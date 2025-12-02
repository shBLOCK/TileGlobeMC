mod block;
pub use block::*;
pub mod blocks;
mod registry;

use core::fmt::Debug;
use defmt_or_log::maybe_derive_format;
pub use registry::*;
use crate::utils::IndexedEnum;

pub type BlockStateType = u16;
#[derive(
    Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, derive_more::From, derive_more::Into,
)]
#[maybe_derive_format]
pub struct BlockState(pub BlockStateType);

impl BlockState {
    pub fn get_block(&self) -> &'static dyn DynifiedBlock {
        Blocks.get_block(self)
    }
}

pub type StateIdType = u16;
#[derive(
    Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, derive_more::From, derive_more::Into,
)]
#[maybe_derive_format]
pub struct StateId(pub StateIdType);

pub trait BlockStateImpl: Sized + Copy + From<BlockState> {
    fn block_state(&self) -> BlockState;

    fn get_block(&self) -> &'static dyn DynifiedBlock {
        self.block_state().get_block()
    }
}

impl<T: BlockStateImpl> From<T> for BlockState {
    fn from(value: T) -> Self {
        value.block_state()
    }
}

pub trait SimpleBlockState: BlockStateImpl {
    fn from_state_id(id: StateId) -> Self;
    fn state_id(&self) -> StateId;
    fn set_state_id(&mut self, id: StateId);
}

pub trait Property<T, const N: StateIdType>: SimpleBlockState {
    const ID_GROUP_SIZE: StateIdType;

    fn get_raw(&self) -> StateIdType {
        self.state_id().0 / Self::ID_GROUP_SIZE % N
    }
    fn set_raw(&mut self, value: StateIdType) {
        let mut id = self.state_id().0;
        id += (value - id / Self::ID_GROUP_SIZE % N) * Self::ID_GROUP_SIZE;
        self.set_state_id(id.into());
    }
    fn get(&self) -> T;
    fn set(&mut self, value: T);
}

pub trait IntProperty<const MIN: i8, const MAX: i8>:
    Property<i8, { (MAX - MIN + 1) as StateIdType }>
{
    fn get(&self) -> i8 {
        (self.get_raw() as i16 + MIN as i16) as i8
    }
    fn set(&mut self, value: i8) {
        self.set_raw((value - MIN) as StateIdType)
    }
}

pub trait BoolProperty<const ID_GROUP_SIZE: StateIdType>: Property<bool, 2> {
    fn get(&self) -> bool {
        self.get_raw() == 0
    }
    fn set(&mut self, value: bool) {
        self.set_raw((!value) as StateIdType)
    }
}

pub trait EnumProperty<T: IndexedEnum<u8>>: Property<T, { T::VARIANTS.len() as StateIdType }> {
    fn get(&self) -> T {
        T::from(self.get_raw() as u8)
    }
    fn set(&mut self, value: T) {
        self.set_raw(value.into() as StateIdType)
    }
}
