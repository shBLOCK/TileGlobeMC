#![no_std]

use embassy_executor::Spawner;

pub mod master_node;
pub use master_node::master_node_start;

pub async fn node_start(spawner: Spawner) {}
