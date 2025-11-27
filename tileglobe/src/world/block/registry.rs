use tileglobe_utils::resloc::ResLoc;
use crate::world::block::block::Block;
use crate::world::block::BlockState;

pub struct Blocks;
impl Blocks {
    // const BLOCKS: [(BlockId, &'static dyn Block); _] = {
    //     let mut blocks: [(BlockId, &'static dyn Block); _] = [
    //         (BlockId(0), &BlockA),
    //         (BlockId(10), &BlockB(1)),
    //         (BlockId(20), &BlockB(2)),
    //     ];
    //
    //     blocks
    // };
    //
    // pub fn get_block(id: BlockId) -> &'static dyn Block {
    //     match Self::BLOCKS.binary_search_by_key(&id, |&(id, _)| id) {
    //         Ok(idx) => Self::BLOCKS[idx].1,
    //         Err(0) => unreachable!(),
    //         Err(idx) => Self::BLOCKS[idx - 1].1,
    //     }
    // }

    pub(super) fn get_block(&self, bs: &BlockState) -> &'static dyn Block {
        todo!()
    }
}

pub struct BlockResLocs;
impl BlockResLocs {
    tileglobe_proc_macro::mc_block_resloc_consts!();
}