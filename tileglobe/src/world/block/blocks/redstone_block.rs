use tileglobe_proc_macro::mc_block_id_base;
use tileglobe_utils::direction::Direction;
use tileglobe_utils::pos::BlockPos;
use tileglobe_utils::resloc::ResLoc;
use crate::world::block::{Block, BlockResLocs, BlockState};
use crate::world::world::_World;

#[derive(Debug)]
pub struct RedstoneBlock;
impl Block for RedstoneBlock {
    fn resloc(&self) -> &'static ResLoc<'static> {
        BlockResLocs::REDSTONE_BLOCK
    }

    fn default_state(&self) -> BlockState {
        BlockState(mc_block_id_base!("redstone_block"))
    }

    async fn get_signal(&self, world: &_World, pos: BlockPos, blockstate: BlockState, direction: Direction) -> u8 {
        15
    }
}