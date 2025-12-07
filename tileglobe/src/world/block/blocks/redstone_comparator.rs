use crate::world::block::blocks::HorizontalDirection;
use crate::world::block::blocks::redstone_repeater::RedstoneRepeaterBlock;
use crate::world::block::blocks::redstone_wire::RedstoneWireBlock;
use crate::world::block::{
    Block, BlockResLocs, BlockState, BlockStateImpl, BoolProperty, EnumProperty, Property,
    SimpleBlockState, StateId,
};
use crate::world::world::{_World, World};
use alloc::collections::BTreeMap;
use core::cmp::max;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use glam::{Vec2, Vec3, Vec3Swizzles};
use ordered_float::OrderedFloat;
use tileglobe_proc_macro::mc_block_id_base;
use tileglobe_utils::direction::Direction;
use tileglobe_utils::indexed_enum::IndexedEnum;
use tileglobe_utils::pos::BlockPos;
use tileglobe_utils::resloc::ResLoc;

static SIGNALS: Mutex<CriticalSectionRawMutex, BTreeMap<BlockPos, u8>> =
    Mutex::new(BTreeMap::new()); // hack! should actually be implemented with blockentity

#[derive(Debug)]
pub struct RedstoneComparatorBlock;
impl RedstoneComparatorBlock {
    pub fn is_redstone_comparator(blockstate: BlockState) -> bool {
        (mc_block_id_base!("comparator")..(mc_block_id_base!("comparator") + 16))
            .contains(&blockstate.0)
    }

    async fn get_input(world: &_World, pos: BlockPos, direction: HorizontalDirection) -> u8 {
        world
            .get_signal(
                pos.offset_dir(direction.direction()),
                direction.direction().opposite(),
            )
            .await
    }

    async fn get_side_input(world: &_World, pos: BlockPos, direction: HorizontalDirection) -> u8 {
        if let Ok(blockstate) = world
            .get_block_state(pos.offset_dir(direction.direction()))
            .await
        {
            if RedstoneWireBlock::is_redstone_wire(blockstate) {
                RedstoneWireBlock
                    .get_signal(world, pos, blockstate, direction.direction().opposite())
                    .await
            } else if Self::is_redstone_comparator(blockstate) {
                Self.get_signal(world, pos, blockstate, direction.direction().opposite())
                    .await
            } else if RedstoneRepeaterBlock::is_redstone_repeater(blockstate) {
                RedstoneRepeaterBlock
                    .get_signal(world, pos, blockstate, direction.direction().opposite())
                    .await
            } else if blockstate.0 == mc_block_id_base!("redstone_block") {
                15
            } else {
                0
            }
        } else {
            0
        }
    }

    async fn calculate_output(world: &_World, pos: BlockPos, state: RedstoneComparatorState) -> u8 {
        let input = Self::get_input(world, pos, state.facing()).await;
        let side_input = max(
            Self::get_side_input(world, pos, state.facing().cw()).await,
            Self::get_side_input(world, pos, state.facing().ccw()).await,
        );
        match state.mode() {
            Mode::COMPARE => {
                if input >= side_input {
                    input
                } else {
                    0
                }
            }
            Mode::SUBTRACT => max(0, input - side_input),
        }
    }
}
impl Block for RedstoneComparatorBlock {
    fn resloc(&self) -> &'static ResLoc<'static> {
        BlockResLocs::COMPARATOR
    }

    fn default_state(&self) -> BlockState {
        BlockState(mc_block_id_base!("comparator") + 1)
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
        let mut state = RedstoneComparatorState::from(self.default_state());

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

    async fn on_destroyed(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        SIGNALS.lock().await.remove(&pos);
    }

    async fn on_use_without_item(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let mut state = RedstoneComparatorState::from(blockstate);
        state.set_mode(if state.mode() == Mode::COMPARE {
            Mode::SUBTRACT
        } else {
            Mode::COMPARE
        });
        if let Ok(_) = world.set_block_state(pos, state.into()).await {
            world.schedule_tick(pos, 2, 0).await;
            world.update_neighbors(pos).await;
        }
    }

    async fn tick(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let mut state = RedstoneComparatorState::from(blockstate);
        let output = Self::calculate_output(world, pos, state).await;

        state.set_powered(output > 0);
        if state.block_state() != blockstate {
            let _ = world.set_block_state(pos, state.into()).await;
        }

        let old_output = SIGNALS.lock().await.insert(pos, output);
        if old_output != Some(output) {
            world.update_neighbors(pos).await;
            world
                .update_neighbors_except_for_direction(
                    pos.offset_dir(state.facing().direction().opposite()),
                    state.facing().direction(),
                )
                .await;
        }
    }

    async fn update(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        if Self::calculate_output(world, pos, blockstate.into()).await
            != SIGNALS.lock().await.get(&pos).cloned().unwrap_or(0)
        {
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
        let state = RedstoneComparatorState::from(blockstate);
        if state.facing().direction().opposite() == direction {
            SIGNALS.lock().await.get(&pos).cloned().unwrap_or(0)
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

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
#[repr(u8)]
enum Mode {
    COMPARE,
    SUBTRACT,
}
impl IndexedEnum for Mode {
    type Index = u8;
    const VARIANTS: &'static [Self] = &[Self::COMPARE, Self::SUBTRACT];
}
impl From<u8> for Mode {
    fn from(value: u8) -> Self {
        Self::variants()[value as usize]
    }
}
impl From<Mode> for u8 {
    fn from(value: Mode) -> Self {
        value as Self
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct RedstoneComparatorState(StateId);
impl BoolProperty<1> for RedstoneComparatorState {} // powered
impl Property<bool, 2, 1> for RedstoneComparatorState {}
impl EnumProperty<Mode, 2> for RedstoneComparatorState {} // mode
impl Property<Mode, 2, 2> for RedstoneComparatorState {}
impl EnumProperty<HorizontalDirection, 4> for RedstoneComparatorState {} // facing
impl Property<HorizontalDirection, 4, 4> for RedstoneComparatorState {}
impl RedstoneComparatorState {
    fn powered(self) -> bool {
        <Self as BoolProperty<1>>::get(self)
    }
    fn set_powered(&mut self, value: bool) {
        <Self as BoolProperty<1>>::set(self, value);
    }

    fn mode(self) -> Mode {
        <Self as EnumProperty<Mode, 2>>::get(self)
    }
    fn set_mode(&mut self, value: Mode) {
        <Self as EnumProperty<Mode, 2>>::set(self, value);
    }

    fn facing(self) -> HorizontalDirection {
        <Self as EnumProperty<HorizontalDirection, 4>>::get(self)
    }
    fn set_facing(&mut self, value: HorizontalDirection) {
        <Self as EnumProperty<HorizontalDirection, 4>>::set(self, value);
    }
}

impl BlockStateImpl for RedstoneComparatorState {
    fn block_state(self) -> BlockState {
        BlockState(mc_block_id_base!("comparator") + self.state_id().0)
    }
}
impl SimpleBlockState for RedstoneComparatorState {
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
impl From<BlockState> for RedstoneComparatorState {
    fn from(value: BlockState) -> Self {
        Self(StateId(value.0 - mc_block_id_base!("comparator")))
    }
}
