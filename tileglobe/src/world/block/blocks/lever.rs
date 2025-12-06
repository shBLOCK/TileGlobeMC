use defmt_or_log::info;
use glam::Vec3;
use crate::world::block::blocks::{AttachFace, HorizontalDirection};
use crate::world::block::{
    Block, BlockResLocs, BlockState, BlockStateImpl, BoolProperty, EnumProperty, Property,
    SimpleBlockState, StateId, StateIdType,
};
use tileglobe_proc_macro::mc_block_id_base;
use tileglobe_utils::direction::Direction;
use tileglobe_utils::pos::BlockPos;
use tileglobe_utils::resloc::ResLoc;
use crate::world::world::{World, _World};

#[derive(Debug)]
pub struct LeverBlock;
impl Block for LeverBlock {
    fn resloc(&self) -> &'static ResLoc<'static> {
        BlockResLocs::LEVER
    }

    fn default_state(&self) -> BlockState {
        BlockState(mc_block_id_base!("lever") + 9)
    }

    async fn get_state_for_placement(&self, world: &_World, pos: BlockPos, face: Direction, cursor_pos: Vec3) -> BlockState {
        let mut state = LeverState::from(self.default_state());
        info!("{:?}", face);
        state.set_facing(match face {
            Direction::NORTH => HorizontalDirection::NORTH,
            Direction::SOUTH => HorizontalDirection::SOUTH,
            Direction::WEST => HorizontalDirection::WEST,
            Direction::EAST => HorizontalDirection::EAST,
            _ => HorizontalDirection::NORTH,
        });
        state.set_face(match face {
            Direction::DOWN => AttachFace::CEILING,
            Direction::UP => AttachFace::FLOOR,
            _ => AttachFace::WALL,
        });
        state.into()
    }

    async fn on_use_without_item(&self, world: &impl World, pos: BlockPos, blockstate: BlockState) {
        let mut state = LeverState::from(blockstate);
        state.set_powered(!state.powered());
        world.set_block_state(pos, state.into()).await.unwrap();
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct LeverState(StateId);
impl Property<bool, 2, 1> for LeverState {}
impl BoolProperty<1> for LeverState {} // powered
impl Property<HorizontalDirection, 4, 2> for LeverState {}
impl EnumProperty<HorizontalDirection, 2> for LeverState {} // facing
impl Property<AttachFace, 3, 8> for LeverState {}
impl EnumProperty<AttachFace, 8> for LeverState {} // face
impl LeverState {
    fn powered(self) -> bool {
        <Self as BoolProperty<1>>::get(self)
    }
    fn set_powered(&mut self, powered: bool) {
        <Self as BoolProperty<1>>::set(self, powered);
    }
    fn facing(self) -> HorizontalDirection {
        <Self as EnumProperty<HorizontalDirection, 2>>::get(self)
    }
    fn set_facing(&mut self, value: HorizontalDirection) {
        <Self as EnumProperty<HorizontalDirection, 2>>::set(self, value)
    }
    fn face(self) -> AttachFace {
        <Self as EnumProperty<AttachFace, 8>>::get(self)
    }
    fn set_face(&mut self, value: AttachFace) {
        <Self as EnumProperty<AttachFace, 8>>::set(self, value)
    }
}

impl BlockStateImpl for LeverState {
    fn block_state(self) -> BlockState {
        BlockState(mc_block_id_base!("lever") + self.state_id().0)
    }
}
impl SimpleBlockState for LeverState {
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
impl From<BlockState> for LeverState {
    fn from(value: BlockState) -> Self {
        Self(StateId(value.0 - mc_block_id_base!("lever")))
    }
}
