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

    async fn on_use_without_item(&self, world: &impl World, pos: BlockPos, blockstate: BlockState) {
    }
}

pub trait DynifiedBlock: Debug + 'static {
    fn resloc(&self) -> &'static ResLoc<'static>;

    fn default_state(&self) -> BlockState;

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
}

impl<BlockImplementor: Block> DynifiedBlock for BlockImplementor {
    fn resloc(&self) -> &'static ResLoc<'static> {
        BlockImplementor::resloc(self)
    }

    fn default_state(&self) -> BlockState {
        BlockImplementor::default_state(self)
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
}
