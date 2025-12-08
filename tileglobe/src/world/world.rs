use crate::world::block::BlockState;
use crate::world::chunk::{Chunk, ChunkSection};
use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use core::cmp::{Ordering, max};
use core::mem::MaybeUninit;
use defmt_or_log::info;
use dynify::Dynify;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex, RawMutex};
use embassy_sync::mutex::{MappedMutexGuard, Mutex, MutexGuard};
use smallvec::SmallVec;
use tileglobe_utils::direction::Direction;
use tileglobe_utils::indexed_enum::IndexedEnum;
use tileglobe_utils::network::{EIOError, MCPacketBuffer, WriteNumPrimitive, WriteVarInt};
use tileglobe_utils::pos::{BlockPos, ChunkPos};

#[allow(async_fn_in_trait)]
pub trait World {
    async fn get_block_state(&self, pos: BlockPos) -> Result<BlockState, ()>;

    async fn set_block_state(&self, pos: BlockPos, state: BlockState) -> Result<BlockState, ()>;

    async fn tick(&self);

    async fn update_block(&self, pos: BlockPos);
    async fn update_neighbors(&self, pos: BlockPos) {
        for &direction in Direction::variants() {
            self.update_block(pos.offset_dir(direction)).await;
        }
    }
    async fn update_neighbors_except_for_direction(&self, pos: BlockPos, except: Direction) {
        for &direction in Direction::variants() {
            if direction != except {
                self.update_block(pos.offset_dir(direction)).await;
            }
        }
    }

    async fn update_block_shape(&self, pos: BlockPos);
    async fn update_neighbors_shape(&self, pos: BlockPos) {
        for &direction in Direction::variants() {
            self.update_block_shape(pos.offset_dir(direction)).await;
        }
    }

    async fn schedule_tick(&self, pos: BlockPos, delay: u8, priority: i8);

    async fn get_signal(&self, pos: BlockPos, direction: Direction) -> u8;

    async fn get_strong_signal(&self, pos: BlockPos, direction: Direction) -> u8;

    async fn get_signal_to(&self, pos: BlockPos) -> u8 {
        let mut signal = 0;
        for &direction in Direction::variants() {
            signal = max(
                signal,
                self.get_signal(pos.offset_dir(direction), direction.opposite())
                    .await,
            );
        }
        signal
    }

    async fn write_net_chunk<W: embedded_io_async::Write>(
        &self,
        pos: ChunkPos,
        writer: &mut W,
    ) -> Result<(), EIOError<W::Error>>;

    async fn gen_blocks_update_packets_and_clear_changes(&self) -> Vec<MCPacketBuffer> {
        Vec::new()
    }
}

// #[cfg(feature = "rp")]
// pub type _World = LocalWorld<embassy_rp::spinlock_mutex::SpinlockRawMutex<1>, -1, -1, 3, 3>;
// #[cfg(not(feature = "rp"))]
// pub type _World = LocalWorld<CriticalSectionRawMutex, -1, -1, 3, 3>; // TODO: NO!
pub type _World = LocalWorld<NoopRawMutex, -1, -1, 3, 3>;

#[derive(Debug, Eq, PartialEq)]
struct BlockTick {
    pos: BlockPos,
    tick: u32,
    priority: i8,
    sequence: u32,
}
impl Ord for BlockTick {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tick
            .cmp(&other.tick)
            .then_with(|| self.priority.cmp(&other.priority).reverse())
            .then_with(|| self.sequence.cmp(&other.sequence))
    }
}
impl PartialOrd for BlockTick {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct BlockTickScheduler {
    ticks: BTreeSet<BlockTick>,
    positions: BTreeSet<BlockPos>,
    sequence: u32,
}
impl BlockTickScheduler {
    fn new() -> Self {
        Self {
            ticks: BTreeSet::new(),
            positions: BTreeSet::new(),
            sequence: 0,
        }
    }

    pub fn schedule(&mut self, pos: BlockPos, tick: u32, priority: i8) {
        if self.positions.insert(pos) {
            self.ticks.insert(BlockTick {
                pos,
                tick,
                priority,
                sequence: self.sequence,
            });
            self.sequence += 1;
        }
    }

    pub fn pop_at_or_before(&mut self, tick: u32) -> Option<BlockTick> {
        if self.ticks.first()?.tick <= tick {
            let block_tick = self.ticks.pop_first().unwrap();
            self.positions.remove(&block_tick.pos);
            Some(block_tick)
        } else {
            None
        }
    }
}

#[dynify::dynify(DynifiedRedstoneOverride)]
pub trait RedstoneOverride {
    async fn redstone_override(
        &mut self,
        world: &_World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
        strong: bool,
    ) -> Option<u8>;
}

pub struct LocalWorld<
    M: RawMutex + 'static,
    const MIN_X: i16,
    const MIN_Y: i16,
    const SIZE_X: usize,
    const SIZE_Y: usize,
> {
    chunks: [[Mutex<M, Option<Chunk>>; SIZE_Y]; SIZE_X],
    tick_number: Mutex<M, u32>,
    block_tick_scheduler: Mutex<M, BlockTickScheduler>,
    pub redstone_override: Option<Mutex<M, Box<dyn DynifiedRedstoneOverride>>>,
    block_updates: Mutex<M, SmallVec<[BlockPos; 1024]>>,
}

impl<M: RawMutex, const MIN_X: i16, const MIN_Y: i16, const SIZE_X: usize, const SIZE_Y: usize>
    LocalWorld<M, MIN_X, MIN_Y, SIZE_X, SIZE_Y>
{
    pub fn new() -> Self {
        Self {
            chunks: core::array::from_fn(|_| core::array::from_fn(|_| Mutex::new(None))),
            tick_number: Mutex::new(0),
            block_tick_scheduler: Mutex::new(BlockTickScheduler::new()),
            redstone_override: None,
            block_updates: Mutex::new(SmallVec::new()),
        }
    }

    fn get_chunk_mutex(&self, pos: ChunkPos) -> Result<&Mutex<M, Option<Chunk>>, ()> {
        Ok(self
            .chunks
            .get((pos.x - MIN_X) as usize)
            .ok_or(())?
            .get((pos.y - MIN_Y) as usize)
            .ok_or(())?)
    }

    pub async fn get_chunk(&self, pos: ChunkPos) -> Result<MappedMutexGuard<'_, M, Chunk>, ()> {
        let locked = self.get_chunk_mutex(pos)?.lock().await;
        if locked.is_none() {
            return Err(());
        }
        Ok(MutexGuard::map(locked, |it| it.as_mut().unwrap()))
    }

    pub async fn set_chunk(&self, pos: ChunkPos, chunk: Chunk) -> Result<(), ()> {
        *self.get_chunk_mutex(pos)?.lock().await = Some(chunk);
        Ok(())
    }

    const fn _min_corner() -> (i16, i16) {
        (MIN_X, MIN_Y)
    }

    const fn _size() -> (usize, usize) {
        (SIZE_X, SIZE_Y)
    }
}

impl _World {
    async fn get_redstone_override(
        &self,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
        strong: bool,
    ) -> Option<u8> {
        if let Some(rso) = &self.redstone_override {
            let mut c = [MaybeUninit::uninit(); 64];
            rso.lock()
                .await
                .redstone_override(self, pos, blockstate, direction, strong)
                .init(&mut c)
                .await
        } else {
            None
        }
    }
}

impl World for _World {
    async fn get_block_state(&self, pos: BlockPos) -> Result<BlockState, ()> {
        self.get_chunk(pos.chunk_pos())
            .await?
            .get_block_state(pos.chunk_local_pos())
    }

    async fn set_block_state(&self, pos: BlockPos, value: BlockState) -> Result<BlockState, ()> {
        self.get_chunk(pos.chunk_pos())
            .await?
            .set_block_state(pos.chunk_local_pos(), value)
    }

    async fn tick(&self) {
        let current_tick = { *self.tick_number.lock().await };

        info!("tick {}", current_tick);

        let mut c = [MaybeUninit::uninit(); 512];
        while let Some(block_tick) = {
            self.block_tick_scheduler
                .lock()
                .await
                .pop_at_or_before(current_tick)
        } {
            if let Ok(blockstate) = self.get_block_state(block_tick.pos).await {
                blockstate
                    .get_block()
                    .tick(self, block_tick.pos, blockstate)
                    .init(&mut c)
                    .await;
            }
        }

        while let Some(pos) = { self.block_updates.lock().await.pop() } {
            if let Ok(blockstate) = self.get_block_state(pos).await {
                let mut c = [MaybeUninit::<u8>::uninit(); 512];
                blockstate
                    .get_block()
                    .update(self, pos, blockstate)
                    .init(&mut c)
                    .await;
                // info!("{:?} {}", blockstate.get_block(), c.capacity());
            }
        }

        *self.tick_number.lock().await += 1;
    }

    async fn update_block(&self, pos: BlockPos) {
        // if let Ok(blockstate) = self.get_block_state(pos).await {
        //     let mut c = SmallVec::<[MaybeUninit<u8>; 1024]>::new();
        //     blockstate
        //         .get_block()
        //         .update(self, pos, blockstate)
        //         .init(&mut c)
        //         .await;
        // }
        self.block_updates.lock().await.push(pos);
    }

    async fn update_block_shape(&self, pos: BlockPos) {
        if let Ok(blockstate) = self.get_block_state(pos).await {
            let mut c = SmallVec::<[MaybeUninit<u8>; 64]>::new();
            blockstate
                .get_block()
                .update_shape(self, pos, blockstate)
                .init(&mut c)
                .await;
        }
    }

    async fn schedule_tick(&self, pos: BlockPos, delay: u8, priority: i8) {
        self.block_tick_scheduler.lock().await.schedule(
            pos,
            *self.tick_number.lock().await + delay as u32,
            priority,
        );
    }

    async fn get_signal(&self, pos: BlockPos, direction: Direction) -> u8 {
        if let Ok(blockstate) = self.get_block_state(pos).await {
            if let Some(signal) = self
                .get_redstone_override(pos, blockstate, direction, false)
                .await
            {
                return signal;
            }

            let mut c = [MaybeUninit::uninit(); 64];
            let block = blockstate.get_block();
            let mut signal = block
                .get_signal(self, pos, blockstate, direction)
                .init(&mut c)
                .await;
            if signal == 0xF {
                return signal;
            }
            if !block.is_redstone_conductor(blockstate) {
                return signal;
            }
            for &neighbor_dir in Direction::variants() {
                if neighbor_dir == direction {
                    continue;
                }
                let neighbor_pos = pos.offset_dir(neighbor_dir);
                let neighbor_signal =
                    if let Ok(blockstate) = self.get_block_state(neighbor_pos).await {
                        blockstate
                            .get_block()
                            .get_signal(self, pos, blockstate, neighbor_dir.opposite())
                            .init(&mut c)
                            .await
                    } else {
                        0
                    };
                signal = max(signal, neighbor_signal);
                if signal == 0xF {
                    return signal;
                }
            }
            signal
        } else {
            0
        }
    }

    async fn get_strong_signal(&self, pos: BlockPos, direction: Direction) -> u8 {
        if let Ok(blockstate) = self.get_block_state(pos).await {
            if let Some(signal) = self
                .get_redstone_override(pos, blockstate, direction, false)
                .await
            {
                return signal;
            }

            let mut signal = 0;
            if !blockstate.get_block().is_redstone_conductor(blockstate) {
                return signal;
            }
            let mut c = [MaybeUninit::uninit(); 64];
            for &neighbor_dir in Direction::variants() {
                if neighbor_dir == direction {
                    continue;
                }
                let neighbor_pos = pos.offset_dir(neighbor_dir);
                let neighbor_signal =
                    if let Ok(blockstate) = self.get_block_state(neighbor_pos).await {
                        blockstate
                            .get_block()
                            .get_strong_signal(self, pos, blockstate, neighbor_dir.opposite())
                            .init(&mut c)
                            .await
                    } else {
                        0
                    };
                signal = max(signal, neighbor_signal);
                if signal == 0xF {
                    return signal;
                }
            }
            signal
        } else {
            0
        }
    }

    async fn write_net_chunk<W: embedded_io_async::Write>(
        &self,
        pos: ChunkPos,
        writer: &mut W,
    ) -> Result<(), EIOError<W::Error>> {
        let chunk = self.get_chunk(pos).await;

        writer.write_varint(0u32).await?; // heightmaps

        let total_size = match chunk.as_ref() {
            Ok(c) => (-4..20)
                .map(|y| c.get_section(y).unwrap().serialized_size() + 1 + 1)
                .sum(),
            Err(_) => (2 + 1 + 1 + 1 + 1) * 24,
        };
        writer.write_varint::<u32>(total_size as u32).await?; // bytes

        for cy in -4..20 {
            // blocks
            let section = match chunk.as_ref() {
                Ok(c) => c.get_section(cy),
                Err(_) => Err(()),
            };
            match section {
                Ok(s) => s.serialize_into(writer).await?,
                Err(_) => {
                    // write empty section
                    writer.write_be(0u16).await?;
                    writer.write_be(0u8).await?;
                    writer.write_varint(0u32).await?;
                }
            };

            // biomes
            writer.write_be(0u8).await?;
            writer.write_varint(0u32).await?;
        }

        writer.write_varint(0u32).await?; // block entities

        // light
        // writer.write_varint(1u32).await?;
        // writer.write_be(0xFFFF_FF00_0000_0000u64).await?;
        // writer.write_varint(1u32).await?;
        // writer.write_be(0x0u64).await?;
        //
        // writer.write_varint(1u32).await?;
        // writer.write_be(0x0u64).await?;
        // writer.write_varint(1u32).await?;
        // writer.write_be(0xFFFF_FF00_0000_0000u64).await?;
        //
        // writer.write_varint(24u32).await?;
        // for _ in 0..24 {
        //     writer.write_varint(2048u32).await?;
        //     for _ in 0..2048 {
        //         writer.write_be(0xFFu8).await?;
        //     }
        // }
        //
        // writer.write_varint(0u32).await?;

        writer.write_varint(1u32).await?;
        writer.write_be(0x0u64).await?;
        writer.write_varint(1u32).await?;
        writer.write_be(0x0u64).await?;

        writer.write_varint(1u32).await?;
        writer.write_be(0xFFFF_FF00_0000_0000u64).await?;
        writer.write_varint(1u32).await?;
        writer.write_be(0xFFFF_FF00_0000_0000u64).await?;

        writer.write_varint(0u32).await?;
        writer.write_varint(0u32).await?;

        Ok(())
    }

    async fn gen_blocks_update_packets_and_clear_changes(&self) -> Vec<MCPacketBuffer> {
        let mut vec = Vec::<MCPacketBuffer>::new();
        let (min_x, min_y) = Self::_min_corner();
        let (size_x, size_y) = Self::_size();
        for x in min_x..(min_x + size_x as i16) {
            for y in min_y..(min_y + size_y as i16) {
                let pos = ChunkPos::new(x, y);
                if let Ok(mut chunk) = self.get_chunk(pos).await {
                    vec.extend(chunk.gen_blocks_update_packets_and_clear_changes(pos).await);
                }
            }
        }
        vec
    }
}
