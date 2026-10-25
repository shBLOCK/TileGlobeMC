use crate::utils::BlockPos;
use crate::utils::RID;
use crate::world::block::{
    Block, BlockRIDs, BlockState, BlockStateImpl, BoolProperty, Property, SimpleBlockState, StateId,
};
use crate::world::utils::mc_block_index_base;
use crate::world::world::{_World, World};

#[derive(Debug)]
pub struct RedstoneLampBlock;
impl Block for RedstoneLampBlock {
    fn rid(&self) -> &'static RID<'static> {
        BlockRIDs::REDSTONE_LAMP
    }

    fn default_state(&self) -> BlockState {
        BlockState(mc_block_index_base!("redstone_lamp") + 1)
    }

    async fn tick(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let mut state = RedstoneLampState::from(blockstate);
        if state.lit() && world.get_signal_to(pos).await == 0 {
            state.set_lit(false);
            if let Ok(_) = world.set_block_state(pos, state.into()).await {
                world.update_neighbors(pos).await;
            }
        }
    }

    async fn update(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let mut state = RedstoneLampState::from(blockstate);
        let has_signal = world.get_signal_to(pos).await != 0;
        if !state.lit() {
            if has_signal {
                state.set_lit(true);
                if let Ok(_) = world.set_block_state(pos, state.into()).await {
                    world.update_neighbors(pos).await;
                }
            }
        } else {
            if !has_signal {
                world.schedule_tick(pos, 4, 0).await;
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct RedstoneLampState(StateId);
impl BoolProperty<1> for RedstoneLampState {} // lit
impl Property<bool, 2, 1> for RedstoneLampState {}
impl RedstoneLampState {
    fn lit(self) -> bool {
        self.get()
    }

    fn set_lit(&mut self, lit: bool) {
        self.set(lit)
    }
}
impl BlockStateImpl for RedstoneLampState {
    fn block_state(self) -> BlockState {
        BlockState(mc_block_index_base!("redstone_lamp") + self.state_id().0)
    }
}
impl SimpleBlockState for RedstoneLampState {
    fn from_state_id(id: StateId) -> Self {
        Self(id)
    }

    fn state_id(self) -> StateId {
        self.0
    }

    fn set_state_id(&mut self, id: StateId) {
        self.0 = id;
    }
}
impl From<BlockState> for RedstoneLampState {
    fn from(value: BlockState) -> Self {
        Self(StateId(value.0 - mc_block_index_base!("redstone_lamp")))
    }
}
