use crate::world::block::blocks::HorizontalDirection;
use crate::world::block::{
    Block, BlockResLocs, BlockState, BlockStateImpl, BoolProperty, EnumProperty, Property,
    SimpleBlockState, StateId,
};
use crate::world::world::{_World, World};
use glam::Vec3;
use tileglobe_proc_macro::mc_block_id_base;
use tileglobe_utils::direction::Direction;
use tileglobe_utils::pos::BlockPos;
use tileglobe_utils::resloc::ResLoc;

#[derive(Debug)]
pub struct RedstoneTorchBlock {
    pub(crate) torch_type: RedstoneTorchType,
}
impl RedstoneTorchBlock {
    async fn update_neighbors(world: &_World, pos: BlockPos) {
        world.update_neighbors(pos).await;
        world
            .update_neighbors_except_for_direction(pos.offset_dir(Direction::UP), Direction::DOWN)
            .await;
    }

    async fn should_output(world: &_World, pos: BlockPos, state: RedstoneTorchState) -> bool {
        let input = world.get_signal(pos.offset_dir(state.facing), state.facing.opposite()).await;
        input == 0
    }
}
impl Block for RedstoneTorchBlock {
    fn resloc(&self) -> &'static ResLoc<'static> {
        match self.torch_type {
            RedstoneTorchType::Floor => BlockResLocs::REDSTONE_TORCH,
            RedstoneTorchType::Wall => BlockResLocs::REDSTONE_WALL_TORCH,
        }
    }

    fn default_state(&self) -> BlockState {
        match self.torch_type {
            RedstoneTorchType::Floor => RedstoneFloorTorchState::default().block_state(),
            RedstoneTorchType::Wall => RedstoneWallTorchState::default().block_state(),
        }
    }

    fn is_attract_redstone_wire_connection(
        &self,
        blockstate: BlockState,
        direction: HorizontalDirection,
    ) -> bool {
        true
    }

    async fn get_state_for_placement(
        &self,
        world: &_World,
        pos: BlockPos,
        face: Direction,
        cursor_pos: Vec3,
    ) -> BlockState {
        let block_face = match face {
            Direction::DOWN | Direction::UP => Direction::DOWN,
            dir => dir.opposite(),
        };
        RedstoneTorchState {
            facing: block_face,
            lit: true,
        }.block_state()
    }

    async fn on_placed(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        world.update_neighbors_shape(pos).await;
        world.update_block(pos).await;
        Self::update_neighbors(world, pos).await;
    }

    async fn on_destroyed(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        world.update_neighbors_shape(pos).await;
        Self::update_neighbors(world, pos).await;
    }

    async fn tick(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let mut state = RedstoneTorchState::from(blockstate);
        state.lit = Self::should_output(world, pos, state).await;
        let new_blockstate = state.block_state();
        if new_blockstate != blockstate {
            if let Ok(_) = world.set_block_state(pos, new_blockstate).await {
                Self::update_neighbors(world, pos).await;
            }
        }
    }

    async fn update(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let state = RedstoneTorchState::from(blockstate);
        if state.lit != Self::should_output(world, pos, state).await {
            world.schedule_tick(pos, 2, 0).await;
        }
    }

    async fn get_signal(
        &self,
        world: &_World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
    ) -> u8 {
        let state = RedstoneTorchState::from(blockstate);
        if state.lit && state.facing != direction {
            15
        } else {
            0
        }
    }

    async fn get_strong_signal(
        &self,
        world: &_World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
    ) -> u8 {
        if direction == Direction::UP && RedstoneTorchState::from(blockstate).lit {
            15
        } else {
            0
        }
    }
}

#[derive(Debug)]
pub enum RedstoneTorchType {
    Floor,
    Wall,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct RedstoneTorchState {
    facing: Direction,
    lit: bool,
}
impl RedstoneTorchState {
    fn block_state(&self) -> BlockState {
        match self.facing {
            Direction::DOWN => {
                let mut state = RedstoneFloorTorchState::default();
                BoolProperty::set(&mut state, self.lit);
                state.block_state()
            }
            Direction::NORTH | Direction::SOUTH | Direction::WEST | Direction::EAST => {
                let mut state = RedstoneWallTorchState::default();
                BoolProperty::set(&mut state, self.lit);
                EnumProperty::set(&mut state, self.facing.opposite().try_into().unwrap());
                state.block_state()
            }
            _ => unreachable!(),
        }
    }
}
impl From<BlockState> for RedstoneTorchState {
    fn from(value: BlockState) -> Self {
        if value.0 < 5918 {
            let state = RedstoneFloorTorchState::from(value);
            Self {
                facing: Direction::DOWN,
                lit: state.get(),
            }
        } else {
            let state = RedstoneWallTorchState::from(value);
            Self {
                facing: EnumProperty::get(state).direction().opposite(),
                lit: BoolProperty::get(state),
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct RedstoneFloorTorchState(StateId);
impl BoolProperty<1> for RedstoneFloorTorchState {} // lit
impl Property<bool, 2, 1> for RedstoneFloorTorchState {}
impl BlockStateImpl for RedstoneFloorTorchState {
    fn block_state(self) -> BlockState {
        BlockState(mc_block_id_base!("redstone_torch") + self.state_id().0)
    }
}
impl SimpleBlockState for RedstoneFloorTorchState {
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
impl From<BlockState> for RedstoneFloorTorchState {
    fn from(value: BlockState) -> Self {
        Self(StateId(value.0 - mc_block_id_base!("redstone_torch")))
    }
}
impl Default for RedstoneFloorTorchState {
    fn default() -> Self {
        Self(StateId(0))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct RedstoneWallTorchState(StateId);
impl BoolProperty<1> for RedstoneWallTorchState {} // lit
impl Property<bool, 2, 1> for RedstoneWallTorchState {}
impl EnumProperty<HorizontalDirection, 2> for RedstoneWallTorchState {} // facing
impl Property<HorizontalDirection, 4, 2> for RedstoneWallTorchState {}
impl BlockStateImpl for RedstoneWallTorchState {
    fn block_state(self) -> BlockState {
        BlockState(mc_block_id_base!("redstone_wall_torch") + self.state_id().0)
    }
}
impl SimpleBlockState for RedstoneWallTorchState {
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
impl From<BlockState> for RedstoneWallTorchState {
    fn from(value: BlockState) -> Self {
        Self(StateId(value.0 - mc_block_id_base!("redstone_wall_torch")))
    }
}
impl Default for RedstoneWallTorchState {
    fn default() -> Self {
        Self(StateId(0))
    }
}
