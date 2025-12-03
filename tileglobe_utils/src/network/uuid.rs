use crate::network::{EIOError, EIOReadExactError, ReadExt};
use uuid::Uuid;

#[allow(async_fn_in_trait)]
pub trait ReadUUID: embedded_io_async::Read {
    async fn read_uuid(mut self: &mut Self) -> Result<Uuid, EIOReadExactError<Self::Error>> {
        Ok(Uuid::from_bytes(self.read_bytes().await?))
    }
}

impl<T: embedded_io_async::Read> ReadUUID for T {}

#[allow(async_fn_in_trait)]
pub trait WriteUUID: embedded_io_async::Write {
    async fn write_uuid(&mut self, uuid: Uuid) -> Result<(), EIOError<Self::Error>> {
        self.write_all(uuid.as_bytes()).await?;
        Ok(())
    }
}

impl<T: embedded_io_async::Write> WriteUUID for T {}
