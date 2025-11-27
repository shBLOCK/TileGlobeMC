use crate::world::block::BlockState;
use crate::world::BlockPos;

struct Dimension {
    
}

impl Dimension {
    async fn get_blockstate(&self, pos: BlockPos) -> BlockState {}
    
    async fn set_blockstate(&self, pos: BlockPos, state: BlockState) -> Result<(), ()> {}
}