use crate::world::block::BlockState;
use crate::world::block::misc::MapColor;
use crate::world::world::{_World, LocalWorld, World};
use core::fmt::Debug;
use glam::Vec3;
use tileglobe_utils::color::RGBA8;
use tileglobe_utils::direction::Direction;
use tileglobe_utils::pos::BlockPos;
use tileglobe_utils::resloc::ResLoc;

#[allow(async_fn_in_trait)]
pub trait Block: Debug + 'static {
    fn resloc(&self) -> &'static ResLoc<'static>;

    fn default_state(&self) -> BlockState;

    fn is_redstone_conductor(&self, blockstate: BlockState) -> bool {
        false
    }

    fn map_color(&self, blockstate: BlockState) -> MapColor {
        MapColor::new(RGBA8::new(0xFF, 0xFF, 0xFF, 0))
    }

    async fn get_state_for_placement(
        &self,
        world: &_World,
        pos: BlockPos,
        face: Direction,
        cursor_pos: Vec3,
    ) -> BlockState {
        self.default_state()
    }

    // async fn on_place(&self) {}

    async fn on_use_without_item(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {}

    async fn tick(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {}

    async fn update(&self, world: &_World, pos: BlockPos, blockstate: BlockState) {}

    async fn get_signal(
        &self,
        world: &_World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
    ) -> u8 {
        0
    }
    async fn get_strong_signal(
        &self,
        world: &_World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
    ) -> u8 {
        0
    }
}

pub trait DynifiedBlock: Debug + 'static {
    fn resloc(&self) -> &'static ResLoc<'static>;

    fn default_state(&self) -> BlockState;

    fn is_redstone_conductor(&self, blockstate: BlockState) -> bool;

    fn get_state_for_placement<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        face: Direction,
        cursor_pos: Vec3,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, Direction, Vec3 => dyn 'dynify + Future<Output = BlockState>)
    where
        'this: 'dynify,
        'world: 'dynify;

    fn on_use_without_item<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        blockstate: BlockState,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, BlockState => dyn 'dynify + Future<Output = ()>)
    where
        'this: 'dynify,
        'world: 'dynify;

    fn tick<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        blockstate: BlockState,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, BlockState => dyn 'dynify + Future<Output = ()>)
    where
        'this: 'dynify,
        'world: 'dynify;

    fn update<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        blockstate: BlockState,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, BlockState => dyn 'dynify + Future<Output = ()>)
    where
        'this: 'dynify,
        'world: 'dynify;

    fn get_signal<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, BlockState, Direction => dyn 'dynify + Future<Output = u8>)
    where
        'this: 'dynify,
        'world: 'dynify;

    fn get_strong_signal<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, BlockState, Direction => dyn 'dynify + Future<Output = u8>)
    where
        'this: 'dynify,
        'world: 'dynify;
}

impl<BlockImplementor: Block> DynifiedBlock for BlockImplementor {
    fn resloc(&self) -> &'static ResLoc<'static> {
        BlockImplementor::resloc(self)
    }

    fn default_state(&self) -> BlockState {
        BlockImplementor::default_state(self)
    }

    fn is_redstone_conductor(&self, blockstate: BlockState) -> bool {
        BlockImplementor::is_redstone_conductor(self, blockstate)
    }

    fn get_state_for_placement<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        face: Direction,
        cursor_pos: Vec3,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, Direction, Vec3 => dyn 'dynify + Future<Output = BlockState>)
    where
        'this: 'dynify,
        'world: 'dynify,
    {
        dynify::from_fn!(
            BlockImplementor::get_state_for_placement,
            self,
            world,
            pos,
            face,
            cursor_pos
        )
    }

    fn on_use_without_item<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        blockstate: BlockState,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, BlockState => dyn 'dynify + Future<Output = ()>)
    where
        'this: 'dynify,
        'world: 'dynify,
    {
        dynify::from_fn!(
            BlockImplementor::on_use_without_item,
            self,
            world,
            pos,
            blockstate
        )
    }

    fn tick<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        blockstate: BlockState,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, BlockState => dyn 'dynify + Future<Output = ()>)
    where
        'this: 'dynify,
        'world: 'dynify,
    {
        dynify::from_fn!(BlockImplementor::tick, self, world, pos, blockstate)
    }

    fn update<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        blockstate: BlockState,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, BlockState => dyn 'dynify + Future<Output = ()>)
    where
        'this: 'dynify,
        'world: 'dynify,
    {
        dynify::from_fn!(BlockImplementor::update, self, world, pos, blockstate)
    }

    fn get_signal<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, BlockState, Direction => dyn 'dynify + Future<Output = u8>)
    where
        'this: 'dynify,
        'world: 'dynify,
    {
        dynify::from_fn!(BlockImplementor::get_signal, self, world, pos, blockstate, direction)
    }
    
    fn get_strong_signal<'this, 'world, 'dynify>(
        &'this self,
        world: &'world _World,
        pos: BlockPos,
        blockstate: BlockState,
        direction: Direction,
    ) -> dynify::Fn!(&'this Self, &'world _World, BlockPos, BlockState, Direction => dyn 'dynify + Future<Output = u8>)
    where
        'this: 'dynify,
        'world: 'dynify,
    {
        dynify::from_fn!(BlockImplementor::get_signal, self, world, pos, blockstate, direction)
    }
}
