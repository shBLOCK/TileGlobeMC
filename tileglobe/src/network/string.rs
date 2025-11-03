use crate::network::{EIOError, ReadVarInt, ReadVarIntError, WriteVarInt};
use alloc::boxed::Box;
use alloc::string::{FromUtf8Error, String};
use core::error::Error;

pub trait ReadUTF8: embedded_io_async::Read {
    async fn read_utf8(mut self: &mut Self) -> Result<String, ReadUTF8Error>
    where
        Self::Error: 'static,
    {
        let length = self.read_varint::<u32>().await.map_err(|err| match err {
            ReadVarIntError::TooBig { .. } => ReadUTF8Error::ProtocolError(err),
            ReadVarIntError::IOError(err) => ReadUTF8Error::IOError(err),
        })?;
        let mut buf = unsafe { Box::<[u8]>::new_uninit_slice(length as usize).assume_init() };
        self.read_exact(buf.as_mut())
            .await
            .map_err(|err| ReadUTF8Error::IOError(err.into()))?;
        Ok(String::from_utf8(buf.into_vec()).map_err(|err| ReadUTF8Error::UnicodeError(err))?)
    }
}

impl<T: embedded_io_async::Read> ReadUTF8 for T {}

#[derive(Debug, derive_more::Display)]
#[display("{self:?}")]
pub enum ReadUTF8Error {
    ProtocolError(ReadVarIntError),
    UnicodeError(FromUtf8Error),
    IOError(Box<dyn Error>),
}

impl Error for ReadUTF8Error {}

pub trait WriteUTF8: embedded_io_async::Write {
    async fn write_utf8(mut self: &mut Self, str: &str) -> Result<(), EIOError<Self::Error>> {
        let data = str.as_bytes();
        self.write_varint::<u32>(data.len() as u32).await?;
        self.write_all(data).await?;
        Ok(())
    }
}

impl<T: embedded_io_async::Write> WriteUTF8 for T {}