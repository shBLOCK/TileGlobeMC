use tileglobe_utils::resloc::ResLoc;
use crate::world::block::{Block, BlockResLocs, BlockState};

#[derive(Debug)]
pub struct RedstoneTorchBlock;
impl Block for RedstoneTorchBlock {
    fn resloc(&self) -> &'static ResLoc<'static> {
        todo!()
    }

    fn default_state(&self) -> BlockState {
        todo!()
    }
}