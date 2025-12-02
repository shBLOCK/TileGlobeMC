use crate::world::block::BlockState;
use core::fmt::Debug;
use tileglobe_utils::resloc::ResLoc;

#[allow(async_fn_in_trait)]
pub trait Block: Debug + 'static {
    fn resloc(&self) -> &'static ResLoc<'static>;

    fn default_state(&self) -> BlockState;

    async fn on_use_without_item(&self) {}
}

pub trait DynifiedBlock {
    fn resloc(&self) -> &'static ResLoc<'static>;

    fn default_state(&self) -> BlockState;

    fn on_use_without_item(&self) -> dynify::Fn!(&Self => dyn '_ + Future<Output = ()>);
}

impl<BlockImplementor: Block> DynifiedBlock for BlockImplementor {
    fn resloc(&self) -> &'static ResLoc<'static> {
        BlockImplementor::resloc(self)
    }

    fn default_state(&self) -> BlockState {
        BlockImplementor::default_state(self)
    }

    fn on_use_without_item(&self) -> dynify::Fn!(&Self => dyn '_ + Future<Output = ()>) {
        dynify::from_fn!(BlockImplementor::on_use_without_item, self)
    }
}
