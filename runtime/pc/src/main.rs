use embassy_executor::{Executor, Spawner};
use log::{info, warn};
use static_cell::StaticCell;

#[embassy_executor::task(pool_size = 3)]
async fn client_task(socket: async_net::TcpStream, addr: async_net::SocketAddr) {
    let mut adapter = embedded_io_adapters::futures_03::FromFutures::new(socket);

    let mut client = tileglobe::master_node::MCClient::new(&mut adapter, Some(addr));
    client._main_task().await;

    let socket = adapter.into_inner();
    if let Err(err) = socket.shutdown(std::net::Shutdown::Both) {
        warn!("Failed to shutdown client socket: {}", err);
    }
}

#[embassy_executor::task]
async fn net_task(spawner: Spawner) {
    let tcp_listener = async_net::TcpListener::bind("127.0.0.1:25565")
        .await
        .unwrap();

    loop {
        match tcp_listener.accept().await {
            Ok((socket, addr)) => {
                info!("TCP accepted: {addr}");
                spawner.spawn(client_task(socket, addr).unwrap());
            }
            Err(err) => warn!("TCP accept failed: {err}"),
        }
    }
}

#[embassy_executor::task]
async fn main_setup(spawner: Spawner) {
    spawner.spawn(net_task(spawner).unwrap());
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .filter_module("async_io", log::LevelFilter::Info)
        .format_timestamp_micros()
        .init();

    static EXECUTOR: StaticCell<Executor> = StaticCell::new();

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(main_setup(spawner).unwrap());
    });
}
