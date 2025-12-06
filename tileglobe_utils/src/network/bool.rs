use crate::network::{EIOReadExactError, ReadNumPrimitive};

pub trait ReadBool: embedded_io_async::Read {
    async fn read_bool(mut self: &mut Self) -> Result<bool, EIOReadExactError<Self::Error>> {
        Ok(self.read_be::<u8>().await? != 0) // TODO: Error when value higher than 1
    }
}
impl<T: embedded_io_async::Read> ReadBool for T {}
