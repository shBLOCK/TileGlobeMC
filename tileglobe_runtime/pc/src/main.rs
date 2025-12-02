use embassy_executor::{Executor, Spawner};
use log::{info, warn};
use static_cell::StaticCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::Timer;
use tileglobe::world::world::LocalWorld;
use tileglobe_server::mc_server::MCServer;
use tileglobe_server::MCClient;

#[embassy_executor::task(pool_size = 3)]
async fn mc_client_task(mc_server: &'static MCServer<'static, _World>, socket: async_net::TcpStream, addr: async_net::SocketAddr) {
    let adapter = embedded_io_adapters::futures_03::FromFutures::new(socket);

    let mut client = MCClient::<CriticalSectionRawMutex, _, _, _>::new(mc_server, adapter.clone(), adapter.clone(), Some(addr));
    client._main_task().await;

    let socket = adapter.into_inner();
    if let Err(err) = socket.shutdown(std::net::Shutdown::Both) {
        warn!("Failed to shutdown client socket: {}", err);
    }
}

type _World = LocalWorld<CriticalSectionRawMutex, -1, -1, 1, 1>;

#[embassy_executor::task(pool_size = 1)]
async fn net_task(spawner: Spawner, mc_server: &'static MCServer<'static, _World>) {
    let tcp_listener = async_net::TcpListener::bind("127.0.0.1:25565")
        .await
        .unwrap();

    loop {
        match tcp_listener.accept().await {
            Ok((socket, addr)) => {
                info!("TCP accepted: {addr}");
                spawner.spawn(mc_client_task(mc_server, socket, addr).unwrap());
            }
            Err(err) => warn!("TCP accept failed: {err}"),
        }
    }
}

static WORLD: StaticCell<_World> = StaticCell::new();
static MC_SERVER: StaticCell<MCServer<'_, _World>> = StaticCell::new();

#[embassy_executor::task(pool_size = 1)]
async fn main_task(spawner: Spawner) {
    let world = WORLD.init(_World::new());
    let mc_server = MC_SERVER.init(MCServer::new(world));
    spawner.spawn(net_task(spawner, mc_server).unwrap());

    loop {
        Timer::after_millis(50).await;
    }
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
        spawner.spawn(main_task(spawner).unwrap());
    });
}
