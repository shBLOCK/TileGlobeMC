#![no_std]

extern crate alloc;

use embassy_executor::Spawner;

pub mod utils;
pub mod world;

pub async fn node_start(spawner: Spawner) {}
