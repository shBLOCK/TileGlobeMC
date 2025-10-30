#![no_std]

use defmt_or_log::*;
use embassy_executor::{Executor, Spawner};
use static_cell::StaticCell;

#[embassy_executor::task]
async fn main_setup(spawner: Spawner) {
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
