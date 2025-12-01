use crate::utils::Dynified;
use crate::world::block::{BlockState, BlockStateImpl};
use core::fmt::{Debug, Display};
use tileglobe_utils::resloc::ResLoc;

pub trait Block: Debug + Dynified<dyn DynifiedBlock> {
    fn resloc(&self) -> &'static ResLoc;

    fn default_state(&self) -> BlockState;

    async fn on_use_without_item(&self) {}
}

pub trait DynifiedBlock {
    fn on_use_without_item(&self) -> dynify::Fn!(&Self => dyn '_ + Future<Output = ()>) {
        dynify::from_fn!(Block::on_use_without_item, self)
    }
}