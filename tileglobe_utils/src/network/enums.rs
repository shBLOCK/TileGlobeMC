use crate::network::{EIOReadExactError, ReadNumPrimitive, WriteNumPrimitive};
use num_traits::{FromBytes, PrimInt, ToBytes, Unsigned};
use crate::indexed_enum::IndexedEnum;

#[allow(async_fn_in_trait)]
pub trait ReadIndexedEnum: embedded_io_async::Read {
    async fn read_indexed_enum<T: IndexedEnum<I>, I: PrimInt + Unsigned + FromBytes<Bytes = [u8; size_of::<I>()]>>(
        mut self: &mut Self,
    ) -> Result<T, EIOReadExactError<Self::Error>> {
        Ok(self.read_be::<I>().await?.into())
    }
}

impl <T: embedded_io_async::Read> ReadIndexedEnum for T {}

#[allow(async_fn_in_trait)]
pub trait WriteIndexedEnum: embedded_io_async::Write {
    async fn write_indexed_enum<T: IndexedEnum<I>, I: PrimInt + Unsigned + ToBytes<Bytes = [u8; size_of::<I>()]>>(
        mut self: &mut Self,
        value: T,
    ) -> Result<(), Self::Error> {
        self.write_be::<I>(value.into()).await
    }
}

impl <T: embedded_io_async::Write> WriteIndexedEnum for T {}