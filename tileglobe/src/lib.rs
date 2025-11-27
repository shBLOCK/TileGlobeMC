#![no_std]

#![feature(generic_const_exprs)]

extern crate alloc;

use embassy_executor::Spawner;

pub mod world;
mod network;
pub mod utils;

pub mod master_node;

pub async fn node_start(spawner: Spawner) {}
