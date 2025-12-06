use embassy_executor::{Executor, Spawner};
use log::{info, warn};
use static_cell::StaticCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Duration, Ticker, Timer};
use tileglobe::world::block::BlockState;
use tileglobe::world::chunk::Chunk;
use tileglobe::world::world::{LocalWorld, World, _World};
use tileglobe_server::mc_server::MCServer;
use tileglobe_server::MCClient;
use tileglobe_utils::network::{MCPacketBuffer, WriteVarInt};
use tileglobe_utils::pos::{ChunkLocalPos, ChunkPos};

#[embassy_executor::task(pool_size = 3)]
async fn mc_client_task(mc_server: &'static MCServer<'static, CriticalSectionRawMutex, _World>, socket: async_net::TcpStream, addr: async_net::SocketAddr) {
    let adapter = embedded_io_adapters::futures_03::FromFutures::new(socket);

    let mut client = MCClient::<CriticalSectionRawMutex, _, _, _>::new(mc_server, adapter.clone(), adapter.clone(), Some(addr));
    let result = client.run().await;
    info!("{} disconnected: {:?}", addr, result);

    let socket = adapter.into_inner();
    if let Err(err) = socket.shutdown(std::net::Shutdown::Both) {
        warn!("Failed to shutdown client socket: {}", err);
    }
}

#[embassy_executor::task(pool_size = 1)]
async fn net_task(spawner: Spawner, mc_server: &'static MCServer<'static, CriticalSectionRawMutex, _World>) {
    let tcp_listener = async_net::TcpListener::bind("169.231.32.189:25565")
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
static MC_SERVER: StaticCell<MCServer<'_, CriticalSectionRawMutex, _World>> = StaticCell::new();

#[embassy_executor::task(pool_size = 1)]
async fn main_task(spawner: Spawner) {
    let world = WORLD.init(_World::new());

    for cz in -1i16..1 {
        for cx in -1i16..1 {
            let mut chunk = Chunk::new(-4..=19);
            for sz in 0..16u8 {
                for sx in 0..16u8 {
                    for y in (-4 * 16)..(19 * 16i16) {
                        let (x, z) = (cx * 16 + sx as i16, cz * 16 + sz as i16);
                        let mut blockstate = BlockState(0);
                        if (-10..=-1).contains(&y) {
                            blockstate = BlockState(10);
                        }
                        if blockstate.0 != 0 {
                            let _ = chunk.set_block_state(ChunkLocalPos::new(sx, y, sz), blockstate);
                        }
                    }
                }
            }
            world.set_chunk(ChunkPos::new(cx, cz), chunk).await.unwrap();
        }
    }

    let mc_server = MC_SERVER.init(MCServer::new(world));

    spawner.spawn(net_task(spawner, mc_server).unwrap());

    let mut tick_ticker = Ticker::every(Duration::from_hz(20));
    loop {
        world.tick().await;
        mc_server.tick().await;
        tick_ticker.next().await;
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
