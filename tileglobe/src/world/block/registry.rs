use crate::world::block::Block;
use crate::world::block::blocks::GenericBlock;
use crate::world::block::{BlockState, BlockStateType};
use tileglobe_utils::resloc::ResLoc;

pub struct Blocks;
impl Blocks {
    const _ID_BASE_TO_BLOCK_SORTED: [(BlockStateType, &'static dyn Block); _] = tileglobe_proc_macro::mc_blocks_registry! {
        generic_block: GenericBlock,
        resloc_consts: BlockResLocs,
        entries: {

        }
    };

    pub(super) fn get_block(&self, bs: &BlockState) -> &'static dyn Block {
        match Self::_ID_BASE_TO_BLOCK_SORTED.binary_search_by_key(&bs.0, |&(id, _)| id) {
            Ok(idx) => Self::_ID_BASE_TO_BLOCK_SORTED[idx].1,
            Err(0) => unreachable!(),
            Err(idx) => Self::_ID_BASE_TO_BLOCK_SORTED[idx - 1].1,
        }
    }
}

pub struct BlockResLocs;
impl BlockResLocs {
    tileglobe_proc_macro::mc_block_resloc_consts!();
}