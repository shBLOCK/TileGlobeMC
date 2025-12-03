use crate::network::{EIOReadExactError, ReadExt};
use num_traits::{FromBytes, ToBytes};

#[allow(async_fn_in_trait)]
pub trait ReadNumPrimitive: embedded_io_async::Read {
    async fn read_le<T: FromBytes<Bytes = [u8; size_of::<T>()]>>(
        mut self: &mut Self,
    ) -> Result<T, EIOReadExactError<Self::Error>>
    where
        [u8; size_of::<T>()]: Sized,
    {
        Ok(T::from_le_bytes(
            &(self.read_bytes::<{ size_of::<T>() }>().await?),
        ))
    }

    async fn read_be<T: FromBytes<Bytes = [u8; size_of::<T>()]>>(
        mut self: &mut Self,
    ) -> Result<T, EIOReadExactError<Self::Error>>
    where
        [u8; size_of::<T>()]: Sized,
    {
        Ok(T::from_be_bytes(
            &(self.read_bytes::<{ size_of::<T>() }>().await?),
        ))
    }
}

impl<T: embedded_io_async::Read> ReadNumPrimitive for T {}

#[allow(async_fn_in_trait)]
pub trait WriteNumPrimitive: embedded_io_async::Write {
    async fn write_le<T: ToBytes<Bytes = [u8; size_of::<T>()]>>(
        &mut self,
        value: T,
    ) -> Result<(), Self::Error> {
        self.write_all(&value.to_le_bytes()).await
    }

    async fn write_be<T: ToBytes<Bytes = [u8; size_of::<T>()]>>(
        &mut self,
        value: T,
    ) -> Result<(), Self::Error> {
        self.write_all(&value.to_be_bytes()).await
    }
}

impl<T: embedded_io_async::Write> WriteNumPrimitive for T {}
