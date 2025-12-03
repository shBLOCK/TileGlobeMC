use crate::network::{EIOError, WriteVarInt};
use alloc::vec::Vec;
use core::convert::Infallible;
use defmt_or_log::debug;

pub struct MCPacketBuffer {
    buffer: Vec<u8>,
}

impl embedded_io_async::ErrorType for MCPacketBuffer {
    type Error = Infallible;
}

impl embedded_io_async::Write for MCPacketBuffer {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        unimplemented!()
    }

    async fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.write(buf).await?;
        Ok(())
    }
}

impl MCPacketBuffer {
    pub async fn with_capacity(packet_type: i32, capacity: usize) -> Self {
        let mut pkt = Self {
            buffer: Vec::with_capacity(capacity),
        };
        pkt.write_varint::<i32>(packet_type).await.unwrap();
        pkt
    }

    pub async fn new(packet_type: i32) -> Self {
        Self::with_capacity(packet_type, 64).await
    }
}

async fn write_all_with_log<TX: embedded_io_async::Write>(tx: &mut TX, buf: &[u8]) -> Result<(), TX::Error> {
    let mut buf = buf;
    while !buf.is_empty() {
        match tx.write(buf).await {
            Ok(0) => panic!("write() returned Ok(0)"),
            Ok(n) => buf = &buf[n..],
            Err(e) => return Err(e),
        }
        debug!("Remaining: {:?}", buf.len());
    }
    Ok(())
}

#[allow(async_fn_in_trait)]
pub trait WriteMCPacket: embedded_io_async::Write {
    async fn write_mc_packet(
        mut self: &mut Self,
        pkt: MCPacketBuffer,
    ) -> Result<(), EIOError<Self::Error>> {
        self.write_varint::<u32>(pkt.buffer.len() as u32).await?;
        write_all_with_log(&mut self, &*pkt.buffer).await?;
        Ok(())
    }
}

impl<T: embedded_io_async::Write> WriteMCPacket for T {}
