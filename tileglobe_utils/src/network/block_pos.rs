use crate::network::{EIOReadExactError, ReadNumPrimitive};
use crate::pos::BlockPos;
use num_traits::PrimInt;

pub trait ReadBlockPos: embedded_io_async::Read {
    async fn read_block_pos(mut self: &mut Self) -> Result<BlockPos, EIOReadExactError<Self::Error>> {
        let packed = self.read_be::<u64>().await?;
        Ok(BlockPos::new(
            packed.signed_shr(38) as i16,
            (packed << 52).signed_shr(52) as i16,
            (packed << 26).signed_shr(38) as i16,
        ))
    }
}

impl<T: embedded_io_async::Read> ReadBlockPos for T {}
