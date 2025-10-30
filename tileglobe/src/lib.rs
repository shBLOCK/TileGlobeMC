#![no_std]
extern crate alloc;

use embassy_executor::Spawner;

pub mod master_node;
mod network;

pub async fn node_start(spawner: Spawner) {}
