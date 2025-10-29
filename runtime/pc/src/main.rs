#![no_std]

use defmt_or_log::*;
use embassy_executor::{Executor, Spawner};
use embassy_net::{Ipv4Address, Ipv4Cidr, StaticConfigV4};
use embassy_net_tuntap::TunTapDevice;
use rand_core::TryRngCore;
use static_cell::StaticCell;

#[embassy_executor::task]
async fn embassy_net_task(mut runner: embassy_net::Runner<'static, TunTapDevice>) {
    runner.run().await;
}

async fn embassy_net_setup(spawner: Spawner) -> embassy_net::Stack<'static> {
    let device = TunTapDevice::new("mctap").unwrap();

    let config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
        dns_servers: heapless::Vec::new(),
        gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
    });

    static RESOURCES: StaticCell<embassy_net::StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        device,
        config,
        RESOURCES.init(embassy_net::StackResources::new()),
        rand_core::OsRng.try_next_u64().unwrap(),
    );

    spawner.spawn(embassy_net_task(runner)).unwrap();

    stack
}

#[embassy_executor::task]
async fn main_setup(spawner: Spawner) {
    let net_stack = embassy_net_setup(spawner).await;

    tileglobe::master_node_start(spawner, net_stack).await;
}

fn main() {
    #[cfg(feature = "log")]
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .filter_module("async_io", log::LevelFilter::Info)
        .format_timestamp_micros()
        .init();

    static EXECUTOR: StaticCell<Executor> = StaticCell::new();

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(main_setup(spawner)).unwrap();
    });
}
