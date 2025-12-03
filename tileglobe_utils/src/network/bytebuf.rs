// use alloc::vec::Vec;
//
// struct ByteBuf(Vec<u8>);
//
// impl embedded_io_async::Write for ByteBuf {
//     async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
//         self.0.extend_from_slice(buf);
//         Ok(buf.len())
//     }
//
//     async fn flush(&mut self) -> Result<(), Self::Error> {
//         unimplemented!()
//     }
//
//     async fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
//         self.write(buf).await?;
//         Ok(())
//     }
// }