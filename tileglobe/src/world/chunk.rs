use crate::world::BlockPos;
use crate::world::block::BlockState;
use alloc::vec::Vec;

pub struct Chunk {
    sections: Vec<ChunkSection>,
    bottom_section: i8,
    //TODO: entities
}

impl Chunk {
    pub fn get_section(&self, y: i8) -> Result<&ChunkSection, ()> {
        self.sections.get((y - self.bottom_section) as usize).ok_or(())
    }

    pub fn get_section_mut(&mut self, y: i8) -> Result<&mut ChunkSection, ()> {
        self.sections.get_mut((y - self.bottom_section) as usize).ok_or(())
    }

    fn section_y_at(block_y: i16) -> i8 {
        block_y.div_euclid(16) as i8
    }

    pub fn get_block_state(&self, local_pos: BlockPos) -> Result<BlockState, ()> {
        Ok(self.get_section(Self::section_y_at(local_pos.y))?.get_block_state(local_pos))
    }

    pub fn set_block_state(&mut self, local_pos: BlockPos, bs: BlockState) -> Result<(), ()> {
        self.get_section_mut(Self::section_y_at(local_pos.y))?.set_block_state(local_pos, bs);
        Ok(())
    }
}

pub struct ChunkSection {
    blocks: [[[BlockState; 16]; 16]; 16],
}

impl ChunkSection {
    pub fn get_block_state(&self, pos: BlockPos) -> BlockState {
        self.blocks[pos.y as usize][pos.z as usize][pos.x as usize]
    }

    pub fn set_block_state(&mut self, pos: BlockPos, bs: BlockState) {
        self.blocks[pos.y as usize][pos.z as usize][pos.x as usize] = bs;
    }
}
