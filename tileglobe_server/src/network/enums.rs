use crate::network::{EIOReadExactError, ReadNumPrimitive, WriteNumPrimitive};
use crate::utils::IndexedEnum;
use num_traits::PrimInt;

pub trait ReadIndexedEnum: embedded_io_async::Read {
    async fn read_indexed_enum<T: IndexedEnum<I>, I: PrimInt>(
        mut self: &mut Self,
    ) -> Result<T, EIOReadExactError<Self::Error>> {
        Ok(self.read_be::<I>().await?.into())
    }
}

impl <T: embedded_io_async::Read> ReadIndexedEnum for T {}

pub trait WriteIndexedEnum: embedded_io_async::Write {
    async fn write_indexed_enum<T: IndexedEnum<I>, I: PrimInt>(
        &mut self,
        value: T,
    ) -> Result<(), Self::Error> {
        self.write_be::<I>(value.into())
    }
}

impl <T: embedded_io_async::Write> WriteIndexedEnum for T {}