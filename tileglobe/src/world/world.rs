use crate::world::block::BlockState;
use crate::world::chunk::Chunk;
use crate::world::{BlockPos, ChunkPos};

pub trait World {
    async fn get_block_state(&self, pos: BlockPos) -> Result<BlockState, ()>;

    async fn set_block_state(&mut self, pos: BlockPos, state: BlockState) -> Result<(), ()>;
}

pub struct LocalWorld<const MIN_X: i16, const MIN_Y: i16, const SIZE_X: i16, const SIZE_Y: i16> {
    chunks: [[Option<Chunk>; (SIZE_Y - MIN_Y) as usize]; (SIZE_X - MIN_X) as usize],
}

impl<const MIN_X: i16, const MIN_Y: i16, const SIZE_X: i16, const SIZE_Y: i16>
    LocalWorld<MIN_X, MIN_Y, SIZE_X, SIZE_Y>
{
    pub fn get_chunk(&self, pos: ChunkPos) -> Result<&Chunk, ()> {
        Ok(self
            .chunks
            .get(pos.x - MIN_X)
            .ok_or(())?
            .get(pos.y - MIN_Y)
            .ok_or(())?
            .ok_or(())?)
    }

    pub async fn tick() {}
}

impl World for LocalWorld<_, _, _, _> {
    async fn get_block_state(&self, pos: BlockPos) -> Result<BlockState, ()> {
        self.get_chunk(pos.chunk_pos())?
            .get_block_state(pos.chunk_local_pos())
    }

    async fn set_block_state(&mut self, pos: BlockPos, state: BlockState) -> Result<(), ()> {
        self.get_chunk(pos.chunk_pos())?
            .set_block_state(pos.chunk_local_pos(), state)
    }
}
