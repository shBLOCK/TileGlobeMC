use crate::world::block::blocks::HorizontalDirection;
use crate::world::block::{
    Block, BlockResLocs, BlockState, BlockStateImpl, BoolProperty, EnumProperty, IntProperty,
    Property, SimpleBlockState, StateId,
};
use crate::world::world::{_World, World};
use glam::{Vec2, Vec3, Vec3Swizzles};
use ordered_float::OrderedFloat;
use tileglobe_proc_macro::mc_block_id_base;
use tileglobe_utils::direction::Direction;
use tileglobe_utils::indexed_enum::IndexedEnum;
use tileglobe_utils::pos::BlockPos;
use tileglobe_utils::resloc::ResLoc;

#[derive(Debug)]
pub struct RedstoneRepeaterBlock;
impl RedstoneRepeaterBlock {
    async fn get_input_signal(world: &_World, pos: BlockPos, blockstate: BlockState) -> u8 {
        let state = RedstoneRepeaterState::from(blockstate);
        world
            .get_signal(
                pos.offset_dir(state.facing().direction()),
                state.facing().direction().opposite(),
            )
            .await
    }
}
impl Block for RedstoneRepeaterBlock {
    fn resloc(&self) -> &'static ResLoc<'static> {
        BlockResLocs::REPEATER
    }

    fn default_state(&self) -> BlockState {
        BlockState(mc_block_id_base!("repeater") + 3)
    }

    fn is_attract_redstone_wire_connection(
        &self,
        blockstate: BlockState,
        direction: HorizontalDirection,
    ) -> bool {
        RedstoneRepeaterState::from(blockstate).facing() == direction
            || RedstoneRepeaterState::from(blockstate)
                .facing()
                .direction()
                .opposite()
                == direction.direction()
    }

    async fn get_state_for_placement(
        &self,
        world: &_World,
        pos: BlockPos,
        face: Direction,
        cursor_pos: Vec3,
    ) -> BlockState {
        let mut state = RedstoneRepeaterState::from(self.default_state());

        state.set_facing(
            HorizontalDirection::variants()
                .iter()
                .max_by_key(|d| {
                    OrderedFloat(
                        d.direction()
                            .normal_i16()
                            .xz()
                            .as_vec2()
                            .dot(cursor_pos.xz() - Vec2::new(0.5, 0.5)),
                    )
                })
                .map(|it| HorizontalDirection::try_from(it.direction().opposite()).unwrap())
                .unwrap(),
        );
        state.block_state()
    }

    async fn on_use_without_item(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let mut state = RedstoneRepeaterState::from(blockstate);
        state.set_delay(if state.delay() == 4 {
            1
        } else {
            state.delay() + 1
        });
        if let Ok(_) = world.set_block_state(pos, state.into()).await {
            world.update_block(pos).await;
        }
    }

    async fn tick(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let mut state = RedstoneRepeaterState::from(blockstate);
        let has_input = Self::get_input_signal(world, pos, blockstate).await > 0;

        if !state.powered() {
            if !has_input {
                world.schedule_tick(pos, (state.delay() * 2) as u8, 0).await;
            }
            state.set_powered(true);
        } else {
            if !has_input {
                state.set_powered(false);
            }
        }

        if state.block_state() != blockstate {
            if let Ok(_) = world.set_block_state(pos, state.into()).await {
                world.update_neighbors(pos).await;
                world
                    .update_neighbors_except_for_direction(
                        pos.offset_dir(state.facing().direction().opposite()),
                        state.facing().direction(),
                    )
                    .await;
            }
        }
    }

    async fn update(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let state = RedstoneRepeaterState::from(blockstate);
        let has_input = Self::get_input_signal(world, pos, blockstate).await > 0;
        if has_input != state.powered() {
            world.schedule_tick(pos, (state.delay() * 2) as u8, 0).await;
        }
    }

    async fn get_signal(
        &self,
        world: &_World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
    ) -> u8 {
        let state = RedstoneRepeaterState::from(blockstate);
        if state.powered() && state.facing().direction().opposite() == direction {
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
        self.get_signal(world, pos, blockstate, direction).await
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct RedstoneRepeaterState(StateId);
impl BoolProperty<1> for RedstoneRepeaterState {} // powered
impl Property<bool, 2, 1> for RedstoneRepeaterState {}
impl BoolProperty<2> for RedstoneRepeaterState {} // locked
impl Property<bool, 2, 2> for RedstoneRepeaterState {}
impl EnumProperty<HorizontalDirection, 4> for RedstoneRepeaterState {} // facing
impl Property<HorizontalDirection, 4, 4> for RedstoneRepeaterState {}
impl IntProperty<1, 4, 16> for RedstoneRepeaterState {} // delay
impl Property<i8, 4, 16> for RedstoneRepeaterState {}
impl RedstoneRepeaterState {
    fn powered(self) -> bool {
        <Self as BoolProperty<1>>::get(self)
    }
    fn set_powered(&mut self, value: bool) {
        <Self as BoolProperty<1>>::set(self, value);
    }
    fn locked(self) -> bool {
        <Self as BoolProperty<2>>::get(self)
    }
    fn set_locked(&mut self, value: bool) {
        <Self as BoolProperty<2>>::set(self, value);
    }
    fn facing(self) -> HorizontalDirection {
        <Self as EnumProperty<HorizontalDirection, 4>>::get(self)
    }
    fn set_facing(&mut self, value: HorizontalDirection) {
        <Self as EnumProperty<HorizontalDirection, 4>>::set(self, value)
    }
    fn delay(self) -> i8 {
        <Self as IntProperty<1, 4, 16>>::get(self)
    }
    fn set_delay(&mut self, value: i8) {
        <Self as IntProperty<1, 4, 16>>::set(self, value)
    }
}

impl BlockStateImpl for RedstoneRepeaterState {
    fn block_state(self) -> BlockState {
        BlockState(mc_block_id_base!("repeater") + self.state_id().0)
    }
}
impl SimpleBlockState for RedstoneRepeaterState {
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
impl From<BlockState> for RedstoneRepeaterState {
    fn from(value: BlockState) -> Self {
        Self(StateId(value.0 - mc_block_id_base!("repeater")))
    }
}
