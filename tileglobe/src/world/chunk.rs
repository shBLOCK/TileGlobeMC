use crate::world::ChunkLocalPos;
use crate::world::block::BlockState;
use alloc::vec::Vec;

pub struct Chunk {
    sections: Vec<ChunkSection>,
    bottom_section: i8,
    //TODO: entities
}

impl Chunk {
    pub fn get_section(&self, y: i8) -> Result<&ChunkSection, ()> {
        self.sections
            .get((y - self.bottom_section) as usize)
            .ok_or(())
    }

    pub fn get_section_mut(&mut self, y: i8) -> Result<&mut ChunkSection, ()> {
        self.sections
            .get_mut((y - self.bottom_section) as usize)
            .ok_or(())
    }

    pub fn get_block_state(&self, pos: ChunkLocalPos) -> Result<BlockState, ()> {
        Ok(self
            .get_section(pos.section())?
            .get_block_state(pos.section_block_index()))
    }

    pub fn set_block_state(
        &mut self,
        pos: ChunkLocalPos,
        bs: BlockState,
    ) -> Result<BlockState, ()> {
        Ok(self
            .get_section_mut(pos.section())?
            .set_block_state(pos.section_block_index(), bs))
    }
}

pub struct ChunkSection {
    blocks: [BlockState; 16 * 16 * 16],
    non_air_blocks: u16,
}

impl ChunkSection {
    pub fn new() -> Self {
        Self {
            blocks: [Default::default(); 16 * 16 * 16],
            non_air_blocks: 0,
        }
    }

    pub fn get_block_state(&self, index: u16) -> BlockState {
        self.blocks[index as usize]
    }

    pub fn set_block_state(&mut self, index: u16, bs: BlockState) -> BlockState {
        let old = self.blocks[index as usize];
        self.blocks[index as usize] = bs;
        if old.is_air() {
            if !bs.is_air() {
                self.non_air_blocks += 1;
            }
        } else {
            if bs.is_air() {
                self.non_air_blocks -= 1;
            }
        }
        old
    }

    pub fn get_data_array(&self) -> &[BlockState; 16 * 16 * 16] {
        &self.blocks
    }

    pub fn non_air_blocks(&self) -> u16 {
        self.non_air_blocks
    }
}
