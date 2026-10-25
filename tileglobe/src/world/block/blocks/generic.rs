use crate::utils::RID;
use crate::world::block::{Block, BlockState, BlockStateType, StateId, StateIdType};

#[derive(derive_more::Debug)]
#[debug("GenericBlock(\"{}\")", self.rid)]
pub struct GenericBlock {
    pub rid: &'static RID<'static>,
    pub index_base: BlockStateType,
    pub num_states: StateIdType,
    pub default_state: StateId,
}

impl Block for GenericBlock {
    fn rid(&self) -> &'static RID<'static> {
        self.rid
    }

    fn default_state(&self) -> BlockState {
        BlockState(self.index_base + self.default_state.0)
    }
}
