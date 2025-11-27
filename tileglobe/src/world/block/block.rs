use core::fmt::Debug;
use tileglobe_utils::resloc::ResLoc;
use crate::utils::Dynified;
use crate::world::block::{BlockState, BlockStateImpl};

#[dynify::dynify]
pub trait Block: Debug + Dynified<dyn DynBlock> {
    fn resloc(&self) -> &'static ResLoc;

    fn default_state(&self) -> BlockState;
}