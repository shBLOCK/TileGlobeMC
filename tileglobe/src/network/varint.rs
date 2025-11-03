#![allow(private_bounds)]

use crate::network::{EIOError, ReadExt};
use alloc::boxed::Box;
use core::error::Error;
use core::fmt::Formatter;
use core::fmt::{Debug, Display};
use core::ops::{BitAnd, BitOrAssign, Shl, ShrAssign};

trait VarIntType<const MAX_BYTES: usize>:
    Copy
    + From<u8>
    + TryInto<u8>
    + Shl<usize, Output = Self>
    + ShrAssign<usize>
    + BitOrAssign
    + BitAnd<Output = Self>
    + Eq
{
}
impl VarIntType<5> for i32 {}
impl VarIntType<5> for i64 {}
impl VarIntType<10> for u32 {}
impl VarIntType<10> for u64 {}

pub trait ReadVarInt<const MAX_BYTES: usize>: embedded_io_async::Read {
    async fn read_varint<I: VarIntType<MAX_BYTES>>(mut self: &mut Self) -> Result<I, ReadVarIntError>
    where
        Self::Error: 'static,
    {
        let mut num: I = 0u8.into();

        for i in 0..MAX_BYTES {
            let byte = self
                .read_bytes::<1>()
                .await
                .map_err(|err| ReadVarIntError::IOError(err.into()))?[0];
            num |= I::from(byte & 0x7F) << (i * 7);
            if byte & 0x80 == 0 {
                return Ok(num);
            }
        }

        Err(ReadVarIntError::TooBig {
            max_bytes: MAX_BYTES,
        })
    }
}

impl<T: embedded_io_async::Read, const MAX_BYTES: usize> ReadVarInt<MAX_BYTES> for T {}

pub trait WriteVarInt<IOE: embedded_io_async::Error, const MAX_BYTES: usize>:
    embedded_io_async::Write<Error = IOE>
{
    async fn write_varint<I: VarIntType<MAX_BYTES>>(
        &mut self,
        num: I,
    ) -> Result<usize, EIOError<IOE>> {
        let mut buf = [0u8; MAX_BYTES];
        let mut num = num;
        for i in 0..MAX_BYTES {
            buf[i] = unsafe { (num & 0x7F.into()).try_into().unwrap_unchecked() };
            num >>= 7;
            if num == 0.into() {
                self.write_all(&buf[..=i]).await?;
                return Ok(i + 1);
            } else {
                buf[i] |= 0x80;
            }
        }
        unreachable!()
    }
}

impl<
    T: embedded_io_async::Write<Error = IOE>,
    IOE: embedded_io_async::Error,
    const MAX_BYTES: usize,
> WriteVarInt<IOE, MAX_BYTES> for T
{
}

#[derive(Debug)]
pub enum ReadVarIntError {
    TooBig { max_bytes: usize },
    IOError(Box<dyn Error>),
}

impl Display for ReadVarIntError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ReadVarIntError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ReadVarIntError::TooBig { .. } => None,
            ReadVarIntError::IOError(err) => Some(err.as_ref()),
        }
    }
}
