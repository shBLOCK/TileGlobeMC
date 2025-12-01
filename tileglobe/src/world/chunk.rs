use crate::world::BlockPos;
use crate::world::block::BlockState;
use alloc::vec::Vec;
use num_traits::Euclid;

pub struct Chunk {
    sections: Vec<ChunkSection>,
    bottom_section: i8,
    //TODO: entities
}

impl Chunk {
    pub fn get_section(&self, y: i8) -> Result<&ChunkSection, ()> {
        self.sections.get(y - self.bottom_section).ok_or(())
    }

    pub fn get_section_at(&self, block_y: i16) -> Result<&ChunkSection, ()> {
        self.get_section(block_y.div_euclid(16) as i8)
    }

    pub fn get_block_state(&self, local_pos: BlockPos) -> Result<BlockState, ()> {
        Ok(self.get_section_at(local_pos.y)?.get_block_state(local_pos))
    }

    pub fn set_block_state(&mut self, local_pos: BlockPos, bs: BlockState) -> Result<(), ()> {
        self.get_section_at(local_pos.y)?.set_block_state(local_pos, bs);
        Ok(())
    }
}

struct ChunkSection {
    blocks: [[[BlockState; 16]; 16]; 16],
}

impl ChunkSection {
    pub fn get_block_state(&self, pos: BlockPos) -> BlockState {
        self.blocks[pos.y][pos.z][pos.x]
    }

    pub fn set_block_state(&self, pos: BlockPos, bs: BlockState) {
        self.blocks[pos.y][pos.z][pos.x] = bs;
    }
}
