use crate::world::block::{
    Block, BlockState, BlockStateType, StateId, StateIdType,
};
use tileglobe_utils::resloc::ResLoc;

#[derive(derive_more::Debug)]
#[debug("GenericBlock(\"{}\")", self.resloc)]
pub struct GenericBlock {
    pub resloc: &'static ResLoc<'static>,
    pub id_base: BlockStateType,
    pub num_states: StateIdType,
    pub default_state: StateId,
}

impl Block for GenericBlock {
    fn resloc(&self) -> &'static ResLoc<'static> {
        self.resloc
    }

    fn default_state(&self) -> BlockState {
        BlockState(self.id_base + self.default_state.0)
    }
}
