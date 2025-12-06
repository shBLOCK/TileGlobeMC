use crate::indexed_enum::IndexedEnum;
use crate::network::{EIOError, EIOReadExactError, ReadNumPrimitive, WriteNumPrimitive};
use num_traits::{FromBytes, ToBytes};

#[allow(async_fn_in_trait)]
pub trait ReadIndexedEnum: embedded_io_async::Read {
    async fn read_indexed_enum<T: IndexedEnum>(
        mut self: &mut Self,
    ) -> Result<T, EIOReadExactError<Self::Error>>
    where
        T::Index: FromBytes<Bytes = [u8; size_of::<T::Index>()]>,
    {
        Ok(self.read_be::<T::Index>().await?.into())
    }
}

impl<T: embedded_io_async::Read> ReadIndexedEnum for T {}

#[allow(async_fn_in_trait)]
pub trait WriteIndexedEnum: embedded_io_async::Write {
    async fn write_indexed_enum<
        T: IndexedEnum
    >(
        mut self: &mut Self,
        value: T,
    ) -> Result<(), EIOError<Self::Error>> where T::Index: ToBytes<Bytes = [u8; size_of::<T::Index>()]> {
        Ok(self.write_be::<T::Index>(value.into()).await?)
    }
}

impl<T: embedded_io_async::Write> WriteIndexedEnum for T {}
