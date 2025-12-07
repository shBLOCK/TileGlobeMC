use crate::world::block::blocks::HorizontalDirection;
use crate::world::block::{
    Block, BlockResLocs, BlockState, BlockStateImpl, EnumProperty, IntProperty, Property,
    SimpleBlockState, StateId,
};
use crate::world::world::{_World, World};
use core::cmp::max;
use core::mem::MaybeUninit;
use dynify::Dynify;
use glam::{I16Vec3, Vec3};
use itertools::iproduct;
use smallvec::SmallVec;
use tileglobe_proc_macro::mc_block_id_base;
use tileglobe_utils::direction::Direction;
use tileglobe_utils::indexed_enum::IndexedEnum;
use tileglobe_utils::pos::BlockPos;
use tileglobe_utils::resloc::ResLoc;

#[derive(Debug)]
pub struct RedstoneWireBlock;
impl RedstoneWireBlock {
    fn is_redstone_wire(blockstate: BlockState) -> bool {
        (mc_block_id_base!("redstone_wire")..(mc_block_id_base!("redstone_wire") + 1296))
            .contains(&blockstate.0)
    }

    async fn get_connection_type_for_side(
        world: &_World,
        pos: BlockPos,
        direction: HorizontalDirection,
        is_up_blocked: bool,
    ) -> Connection {
        if !is_up_blocked {
            let up_is_wire = world
                .get_block_state(
                    pos.offset_dir(direction.direction())
                        .offset_dir(Direction::UP),
                )
                .await
                .map(Self::is_redstone_wire)
                .unwrap_or(false);
            if up_is_wire {
                return Connection::UP;
            }
        }
        let side_blockstate = world
            .get_block_state(pos.offset_dir(direction.direction()))
            .await;

        let side_want_connection = if let Ok(blockstate) = side_blockstate {
            blockstate.get_block().is_attract_redstone_wire_connection(
                blockstate,
                direction.direction().opposite().try_into().unwrap(),
            )
        } else {
            false
        };
        if side_want_connection {
            return Connection::SIDE;
        }

        let down_blocked = if let Ok(blockstate) = side_blockstate {
            blockstate.get_block().is_redstone_conductor(blockstate)
        } else {false};
        if !down_blocked {
            let down_is_wire = world
                .get_block_state(
                    pos.offset_dir(direction.direction())
                        .offset_dir(Direction::DOWN),
                )
                .await
                .map(Self::is_redstone_wire)
                .unwrap_or(false);
            if down_is_wire {
                return Connection::SIDE;
            }
        }

        Connection::NONE
    }

    async fn update_state_shape(
        world: &_World,
        pos: BlockPos,
        old_state: RedstoneWireState,
    ) -> RedstoneWireState {
        let mut state = old_state;

        let is_up_blocked =
            if let Ok(blockstate) = world.get_block_state(pos.offset_dir(Direction::UP)).await {
                blockstate.get_block().is_redstone_conductor(blockstate) // tmp, should be is_solid or something
            } else {
                false
            };
        for &direction in HorizontalDirection::variants() {
            state.set_connection(
                direction,
                Self::get_connection_type_for_side(world, pos, direction, is_up_blocked).await,
            );
        }

        if !old_state.is_dot() && state.is_dot() {
            // set to star
            for &direction in HorizontalDirection::variants() {
                state.set_connection(direction, Connection::SIDE);
            }
            return state;
        }

        let mut connected_sides = HorizontalDirection::variants()
            .iter()
            .filter(|&&d| state.connection(d) != Connection::NONE);
        if let Some(&first_connected_side) = connected_sides.next() {
            if connected_sides.next() == None {
                // only one side connected
                state.set_connection(
                    first_connected_side
                        .direction()
                        .opposite()
                        .try_into()
                        .unwrap(),
                    Connection::SIDE,
                );
            }
        }

        state
    }

    fn update_positions(pos: BlockPos) -> impl Iterator<Item = BlockPos> {
        iproduct!(-2i16..=2, -2i16..=2, -2i16..=2)
            .filter(|(x, y, z)| {
                x.abs() + y.abs() + z.abs() <= 2 && !(*x == 0 && *y == 0 && *z == 0)
            })
            .map(move |(x, y, z)| BlockPos::from(*pos + I16Vec3::new(x, y, z)))
    }

    async fn update_neighbors(world: &_World, pos: BlockPos) {
        for update_pos in Self::update_positions(pos) {
            world.update_block(update_pos).await;
        }
    }

    async fn update_neighbors_shape(world: &_World, pos: BlockPos) {
        for update_pos in Self::update_positions(pos) {
            world.update_block_shape(update_pos).await;
        }
    }

    async fn calculate_power(world: &_World, pos: BlockPos, state: RedstoneWireState) -> u8 {
        let mut c = SmallVec::<[MaybeUninit<u8>; 64]>::new();
        let mut power = 0u8;

        // strong signals
        for &direction in Direction::variants() {
            power = max(
                power,
                world
                    .get_strong_signal(pos.offset_dir(direction), direction.opposite())
                    .await,
            );
            if power == 0xF {
                return power;
            }
        }

        // normal signals
        for &direction in HorizontalDirection::variants() {
            if state.connection(direction) != Connection::SIDE {
                continue;
            }
            if let Ok(blockstate) = world
                .get_block_state(pos.offset_dir(direction.direction()))
                .await
            {
                if !Self::is_redstone_wire(blockstate) {
                    power = max(
                        power,
                        blockstate
                            .get_block()
                            .get_signal(
                                world,
                                pos.offset_dir(direction.direction()),
                                blockstate,
                                direction.direction().opposite(),
                            )
                            .init(&mut c)
                            .await,
                    );
                    if power == 0xF {
                        return power;
                    }
                }
            }
        }

        // wire signals
        for &direction in HorizontalDirection::variants() {
            let wire_pos = match state.connection(direction) {
                Connection::SIDE => Some(pos.offset_dir(direction.direction())),
                Connection::UP => Some(
                    pos.offset_dir(direction.direction())
                        .offset_dir(Direction::UP),
                ),
                Connection::NONE => None,
            };
            if let Some(wire_pos) = wire_pos {
                if let Ok(blockstate) = world.get_block_state(wire_pos).await {
                    if Self::is_redstone_wire(blockstate) {
                        power = max(
                            power,
                            (RedstoneWireState::from(blockstate).power() as u8).saturating_sub(1),
                        );
                    }
                }
            }
        }

        power
    }
}
impl Block for RedstoneWireBlock {
    fn resloc(&self) -> &'static ResLoc<'static> {
        BlockResLocs::REDSTONE_WIRE
    }

    fn default_state(&self) -> BlockState {
        RedstoneWireState::default().block_state()
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
        Self::update_state_shape(world, pos, RedstoneWireState::default())
            .await
            .block_state()
    }

    async fn on_placed(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        Self::update_neighbors_shape(world, pos).await;
        world.update_block(pos).await;
        Self::update_neighbors(world, pos).await;
    }

    async fn on_destroyed(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        Self::update_neighbors_shape(world, pos).await;
        Self::update_neighbors(world, pos).await;
    }

    async fn on_use_without_item(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let mut state = RedstoneWireState::from(blockstate);
        let is_dot = state.is_dot();
        let is_star = state.is_star();
        if is_dot {
            HorizontalDirection::variants()
                .iter()
                .for_each(|it| state.set_connection(*it, Connection::SIDE))
        }
        if is_star {
            HorizontalDirection::variants()
                .iter()
                .for_each(|it| state.set_connection(*it, Connection::NONE))
        }
        if state.block_state() != blockstate {
            if let Ok(_) = world.set_block_state(pos, state.block_state()).await {
                Self::update_neighbors(world, pos).await;
            }
        }
    }

    async fn update(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let state = RedstoneWireState::from(blockstate);
        let power = Self::calculate_power(world, pos, state).await;
        if state.power() != power as i8 {
            let mut new_state = state;
            new_state.set_power(power as i8);
            if let Ok(_) = world.set_block_state(pos, new_state.block_state()).await {
                Self::update_neighbors(world, pos).await;
            }
        }
    }

    async fn update_shape(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {
        let new_state = Self::update_state_shape(world, pos, blockstate.into()).await;
        if new_state.block_state() != blockstate {
            if let Ok(_) = world.set_block_state(pos, new_state.block_state()).await {
                Self::update_neighbors_shape(world, pos).await;
                Self::update_neighbors(world, pos).await;
            }
        }
    }

    async fn get_signal(
        &self,
        world: &_World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
    ) -> u8 {
        let state = RedstoneWireState::from(blockstate);
        if let Ok(horizontal) = HorizontalDirection::try_from(direction) {
            if state.connection(horizontal) == Connection::SIDE {
                state.power() as u8
            } else {
                0
            }
        } else if direction == Direction::DOWN {
            state.power() as u8
        } else {
            0
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
#[repr(u8)]
enum Connection {
    UP,
    SIDE,
    NONE,
}
impl IndexedEnum for Connection {
    type Index = u8;
    const VARIANTS: &'static [Self] = &[Self::UP, Self::SIDE, Self::NONE];
}
impl From<u8> for Connection {
    fn from(value: u8) -> Self {
        Self::variants()[value as usize]
    }
}
impl From<Connection> for u8 {
    fn from(value: Connection) -> Self {
        value as Self
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct RedstoneWireState(StateId);
impl EnumProperty<Connection, 1> for RedstoneWireState {} // west
impl Property<Connection, 3, 1> for RedstoneWireState {}
impl EnumProperty<Connection, 3> for RedstoneWireState {} // south
impl Property<Connection, 3, 3> for RedstoneWireState {}
impl EnumProperty<Connection, 144> for RedstoneWireState {} // north
impl Property<Connection, 3, 144> for RedstoneWireState {}
impl EnumProperty<Connection, 432> for RedstoneWireState {} // east
impl Property<Connection, 3, 432> for RedstoneWireState {}
impl IntProperty<0, 15, 9> for RedstoneWireState {} // power
impl Property<i8, 16, 9> for RedstoneWireState {}
impl RedstoneWireState {
    fn power(self) -> i8 {
        <Self as IntProperty<0, 15, 9>>::get(self)
    }
    fn set_power(&mut self, value: i8) {
        <Self as IntProperty<0, 15, 9>>::set(self, value);
    }

    fn connection(self, direction: HorizontalDirection) -> Connection {
        match direction {
            HorizontalDirection::NORTH => <Self as EnumProperty<Connection, 144>>::get(self),
            HorizontalDirection::SOUTH => <Self as EnumProperty<Connection, 3>>::get(self),
            HorizontalDirection::WEST => <Self as EnumProperty<Connection, 1>>::get(self),
            HorizontalDirection::EAST => <Self as EnumProperty<Connection, 432>>::get(self),
        }
    }
    fn set_connection(&mut self, direction: HorizontalDirection, value: Connection) {
        match direction {
            HorizontalDirection::NORTH => <Self as EnumProperty<Connection, 144>>::set(self, value),
            HorizontalDirection::SOUTH => <Self as EnumProperty<Connection, 3>>::set(self, value),
            HorizontalDirection::WEST => <Self as EnumProperty<Connection, 1>>::set(self, value),
            HorizontalDirection::EAST => <Self as EnumProperty<Connection, 432>>::set(self, value),
        }
    }

    fn is_dot(self) -> bool {
        HorizontalDirection::variants()
            .iter()
            .all(|&d| self.connection(d) == Connection::NONE)
    }

    fn is_star(self) -> bool {
        HorizontalDirection::variants()
            .iter()
            .all(|&d| self.connection(d) == Connection::SIDE)
    }
}

impl Default for RedstoneWireState {
    fn default() -> Self {
        Self(StateId(1160))
    }
}
impl BlockStateImpl for RedstoneWireState {
    fn block_state(self) -> BlockState {
        BlockState(mc_block_id_base!("redstone_wire") + self.state_id().0)
    }
}
impl SimpleBlockState for RedstoneWireState {
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
impl From<BlockState> for RedstoneWireState {
    fn from(value: BlockState) -> Self {
        Self(StateId(value.0 - mc_block_id_base!("redstone_wire")))
    }
}
