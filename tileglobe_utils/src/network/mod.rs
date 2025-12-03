mod mc_packet;
mod primitives;
mod string;
mod uuid;
mod varint;
mod enums;
mod bitbuf;
mod bytebuf;

pub use error_wrappers::*;
pub use mc_packet::*;
pub use primitives::*;
pub use string::*;
pub use uuid::*;
pub use varint::*;
pub use enums::*;
pub use bitbuf::*;

use core::mem::MaybeUninit;

#[allow(async_fn_in_trait)]
pub trait ReadExt: embedded_io_async::Read {
    async fn read_bytes<const BYTES: usize>(
        &mut self,
    ) -> Result<[u8; BYTES], EIOReadExactError<Self::Error>> {
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

impl<T: embedded_io_async::Read> ReadExt for T {}

mod error_wrappers {
    use core::error::Error;
    use core::fmt;
    use core::fmt::{Debug, Formatter};

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
    
    // pub trait WrapEIOError<T, E: embedded_io_async::Error> {
    //     fn wrap_ioe(self) -> Result<T, EIOError<E>>;
    // }
    // 
    // impl<T, E: embedded_io_async::Error> WrapEIOError<T, E> for Result<T, E> {
    //     fn wrap_ioe(self) -> Result<T, EIOError<E>> {
    //         self.map_err(|e| EIOError::from(e))
    //     }
    // }

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
