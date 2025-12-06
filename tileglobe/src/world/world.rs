use crate::world::block::BlockState;
use crate::world::chunk::{Chunk, ChunkSection};
use alloc::vec::Vec;
use defmt_or_log::info;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex};
use embassy_sync::mutex::{MappedMutexGuard, Mutex, MutexGuard};
use tileglobe_utils::network::{EIOError, MCPacketBuffer, WriteNumPrimitive, WriteVarInt};
use tileglobe_utils::pos::{BlockPos, ChunkPos};

#[allow(async_fn_in_trait)]
pub trait World {
    async fn get_block_state(&self, pos: BlockPos) -> Result<BlockState, ()>;

    async fn set_block_state(&self, pos: BlockPos, state: BlockState) -> Result<BlockState, ()>;

    async fn tick(&self);

    async fn write_net_chunk<W: embedded_io_async::Write>(
        &self,
        pos: ChunkPos,
        writer: &mut W,
    ) -> Result<(), EIOError<W::Error>>;

    async fn gen_blocks_update_packets_and_clear_changes(
        &self,
    ) -> Vec<MCPacketBuffer> {
        Vec::new()
    }
}

pub type _World = LocalWorld<CriticalSectionRawMutex, -1, -1, 3, 3>; // TODO: NO!

pub struct LocalWorld<
    M: RawMutex,
    const MIN_X: i16,
    const MIN_Y: i16,
    const SIZE_X: usize,
    const SIZE_Y: usize,
> {
    chunks: [[Mutex<M, Option<Chunk>>; SIZE_Y]; SIZE_X],
}

impl<M: RawMutex, const MIN_X: i16, const MIN_Y: i16, const SIZE_X: usize, const SIZE_Y: usize>
    LocalWorld<M, MIN_X, MIN_Y, SIZE_X, SIZE_Y>
{
    pub fn new() -> Self {
        Self {
            chunks: core::array::from_fn(|_| core::array::from_fn(|_| Mutex::new(None))),
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
}

impl<M: RawMutex, const MIN_X: i16, const MIN_Y: i16, const SIZE_X: usize, const SIZE_Y: usize>
    World for LocalWorld<M, MIN_X, MIN_Y, SIZE_X, SIZE_Y>
{
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

    async fn tick(&self) {}

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
        for x in MIN_X..(MIN_X + SIZE_X as i16) {
            for y in MIN_Y..(MIN_Y + SIZE_Y as i16) {
                let pos = ChunkPos::new(x, y);
                if let Ok(mut chunk) = self.get_chunk(pos).await {
                    vec.extend(chunk.gen_blocks_update_packets_and_clear_changes(pos).await);
                }
            }
        }
        vec
    }
}
