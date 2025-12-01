use crate::world::block::{
    Block, BlockState, BlockStateImpl, BlockStateType, StateId, StateIdType,
};
use tileglobe_utils::resloc::ResLoc;

pub struct GenericBlock {
    resloc: &'static ResLoc<'static>,
    id_base: BlockStateType,
    num_states: StateIdType,
    default_state: StateId,
}

impl Block for GenericBlock {
    fn resloc(&self) -> &'static ResLoc {
        self.resloc
    }

    fn default_state(&self) -> BlockState {
        BlockState(self.id_base + self.default_state.0)
    }
}
