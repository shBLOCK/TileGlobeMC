use alloc::vec::Vec;
use core::cmp::min;
use core::iter::repeat_n;
use num_traits::PrimInt;
use crate::network::{EIOError, WriteNumPrimitive, WriteVarInt};

pub struct BitBuf {
    buf: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
}
impl BitBuf {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity.div_ceil(8)),
            read_pos: 0,
            write_pos: 0,
        }
    }

    pub fn write_int_as_size<I: PrimInt>(&mut self, value: I, as_size: u8) {
        if as_size != 0 {
            let actual_size = size_of::<I>() as u8 * 8;
            let additional_bytes = as_size.saturating_sub((8 - (self.write_pos & 0b111) as u8) & 0b111).div_ceil(8);
            self.buf.reserve(additional_bytes as usize);
            self.buf.extend(repeat_n(0, additional_bytes as usize));
            self.write_pos += as_size as usize;
            let mut value = value;
            let mut pos = self.write_pos - 1;
            let mut remaining = as_size;
            loop {
                let pos_byte = pos >> 3;
                let pos_bit = (pos & 0b111) as u8;
                let start_bit = 7 - pos_bit;
                let end_bit = min(start_bit + remaining, 8);
                let bits = end_bit - start_bit;

                let sub_value = (value << (actual_size - bits) as usize
                    >> (actual_size - bits) as usize)
                    .to_u8()
                    .unwrap();
                self.buf[pos_byte] |= sub_value << start_bit;
                value = value >> bits as usize;

                remaining -= bits;
                if remaining == 0 {
                    break;
                }
                pos -= bits as usize;
            }
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait WriteBitBuf: embedded_io_async::Write {
    async fn write_bit_buf(mut self: &mut Self, bit_buf: &BitBuf) -> Result<(), EIOError<Self::Error>> {
        let data = bit_buf.buf.as_slice();
        let longs = data.len().div_ceil(8);
        self.write_varint(longs as u32).await?;
        self.write_all(data).await?;
        for _ in 0..(data.len() - longs * 8) {
            self.write_be(0u8).await?;
        }
        Ok(())
    }

    async fn write_fixed_bit_buf() {
        todo!()
    }
}

impl<T: embedded_io_async::Write> WriteBitBuf for T {}
