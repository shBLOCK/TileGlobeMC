mod mcclient;

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::signal::Signal;





async fn process_client_packet(reader: &mut impl embedded_io_async::Read) {

}

async fn client_task<M: RawMutex>(
    reader: &mut impl embedded_io_async::Read,
    writer: &mut impl embedded_io_async::Write,
    disconnected: Signal<M, ()>,
) {

}