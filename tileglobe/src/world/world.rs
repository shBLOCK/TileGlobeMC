use crate::world::block::BlockState;
use crate::world::chunk::Chunk;
use crate::world::{BlockPos, ChunkPos};
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::mutex::{MappedMutexGuard, Mutex, MutexGuard};

#[allow(async_fn_in_trait)]
pub trait World {
    async fn get_block_state(&self, pos: BlockPos) -> Result<BlockState, ()>;

    async fn set_block_state(&self, pos: BlockPos, state: BlockState) -> Result<(), ()>;

    async fn tick(&self);
}

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

    pub async fn get_chunk(&self, pos: ChunkPos) -> Result<MappedMutexGuard<'_, M, Chunk>, ()> {
        let mutex = self
            .chunks
            .get((pos.x - MIN_X) as usize)
            .ok_or(())?
            .get((pos.y - MIN_Y) as usize)
            .ok_or(())?;
        let locked = mutex.lock().await;
        if locked.is_none() {
            return Err(());
        }
        Ok(MutexGuard::map(locked, |it| it.as_mut().unwrap()))
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

    async fn set_block_state(&self, pos: BlockPos, state: BlockState) -> Result<(), ()> {
        self.get_chunk(pos.chunk_pos())
            .await?
            .set_block_state(pos.chunk_local_pos(), state)
    }

    async fn tick(&self) {}
}
