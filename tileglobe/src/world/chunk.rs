use alloc::vec::Vec;
use nalgebra::Vector3;
use crate::world::block::BlockState;
use crate::world::BlockPos;

struct Chunk {
    sections: Vec<ChunkSection>,
    bottom_section: i8
}

impl Chunk {
    pub fn get_section(&self, y: i8) -> Option<&ChunkSection> {
        self.sections[y - self.bottom_section]
    }

    pub fn get_blockstate(&self, pos: BlockPos) -> BlockState {

    }
}

struct ChunkSection {
    blocks: [[[BlockState; 16]; 16]; 16]
}

impl ChunkSection {
    pub fn get_blockstate(&self, pos: Vector3<u8>) {
        self.blocks[pos.y][pos.z][pos.x]
    }

    pub fn set_blockstate(&mut self, pos: Vector3<u8>, block: BlockState) {
        self.blocks[pos.y][pos.z][pos.x] = block;
    }
}