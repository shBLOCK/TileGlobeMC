use crate::utils::BlockPos;
use crate::utils::Direction;
use crate::utils::RID;
use crate::world::block::blocks::HorizontalDirection;
use crate::world::block::{Block, BlockRIDs, BlockState};
use crate::world::utils::mc_block_index_base;
use crate::world::world::_World;

#[derive(Debug)]
pub struct RedstoneBlock;
impl Block for RedstoneBlock {
    fn rid(&self) -> &'static RID<'static> {
        BlockRIDs::REDSTONE_BLOCK
    }

    fn default_state(&self) -> BlockState {
        BlockState(mc_block_index_base!("redstone_block"))
    }

    fn is_attract_redstone_wire_connection(
        &self,
        blockstate: BlockState,
        direction: HorizontalDirection,
    ) -> bool {
        true
    }

    async fn get_signal(
        &self,
        world: &_World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
    ) -> u8 {
        15
    }
}
