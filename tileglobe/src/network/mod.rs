mod varint;
mod string;
mod primitives;
mod uuid;
mod mcpacket;

pub use varint::*;
pub use string::*;
pub use primitives::*;
pub use uuid::*;
pub use mcpacket::*;
pub use error_wrappers::*;

use core::mem::MaybeUninit;

pub trait ReadExt: embedded_io_async::Read {
    async fn read_bytes<const BYTES: usize>(&mut self) -> Result<[u8; BYTES], EIOReadExactError<Self::Error>> {
        let mut buf = unsafe { MaybeUninit::<[u8; BYTES]>::uninit().assume_init() };
        self.read_exact(&mut buf).await?;
        Ok(buf)
    }

    async fn skip_bytes(&mut self, n: usize) -> Result<(), EIOReadExactError<Self::Error>> {
        let mut buf = [0u8; 1];
        for _ in 0..n {
            self.read_exact(&mut buf[..]).await?;
        }
        Ok(())
    }
}

impl <T: embedded_io_async::Read> ReadExt for T {}

mod error_wrappers {
    use core::error::Error;
    use core::fmt;
    use core::fmt::{Debug, Display, Formatter};

    // Wrapper to support core::error::Error
    #[derive(derive_more::Display)]
    #[display("{self:?}")]
    pub struct EIOError<E: embedded_io_async::Error>(pub E);

    impl<E: embedded_io_async::Error> From<E> for EIOError<E> {
        fn from(value: E) -> Self {
            Self(value)
        }
    }

    impl<E: embedded_io_async::Error> Debug for EIOError<E> {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }

    impl<E: embedded_io_async::Error> Error for EIOError<E> {}

    // Wrapper to support core::error::Error
    #[derive(derive_more::Display)]
    #[display("{self:?}")]
    pub struct EIOReadExactError<E: Debug>(pub embedded_io_async::ReadExactError<E>);

    impl<E: Debug> From<embedded_io_async::ReadExactError<E>> for EIOReadExactError<E> {
        fn from(value: embedded_io_async::ReadExactError<E>) -> Self {
            Self(value)
        }
    }

    impl<E: Debug> Debug for EIOReadExactError<E> {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }

    impl<E: Debug> Error for EIOReadExactError<E> {}
}