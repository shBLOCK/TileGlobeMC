use core::ops::{BitAnd, BitOrAssign, Shl, ShrAssign};
use embedded_io_async::ReadExactError;

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
    async fn read_varint<IOE: embedded_io_async::Error>(
        reader: &mut impl embedded_io_async::Read<Error = IOE>,
    ) -> Result<Self, VarIntError<ReadExactError<IOE>>> {
        let mut num: Self = 0u8.into();
        let mut buf = [0u8; 1];

        for i in 0..MAX_BYTES {
            reader
                .read_exact(&mut buf[..])
                .await
                .map_err(|err| VarIntError::Other(err))?;

            let byte = buf[0];
            num |= Self::from(byte & 0x7F) << (i * 7);
            if byte & 0x80 == 0 {
                return Ok(num);
            }
        }

        Err(VarIntError::TooBig {
            max_bytes: MAX_BYTES,
        })
    }

    async fn write_varint<IOE: embedded_io_async::Error>(
        &self,
        writer: &mut impl embedded_io_async::Write<Error = IOE>,
    ) -> Result<usize, IOE> {
        let mut buf = [0u8; MAX_BYTES];
        let mut num = *self;
        for i in 0..MAX_BYTES {
            buf[i] = unsafe { (num & 0x7F.into()).try_into().unwrap_unchecked() };
            num >>= 7;
            if num == 0.into() {
                writer.write_all(&buf[..=i]).await?;
                return Ok(i + 1);
            } else {
                buf[i] |= 0x80;
            }
        }
        unreachable!()
    }
}

impl VarIntType<5> for i32 {}
impl VarIntType<5> for i64 {}
impl VarIntType<10> for u32 {}
impl VarIntType<10> for u64 {}

enum VarIntError<E> {
    TooBig { max_bytes: usize },
    Other(E),
}
