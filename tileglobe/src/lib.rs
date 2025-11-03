#![no_std]

#![feature(generic_const_exprs)]

extern crate alloc;

use embassy_executor::Spawner;

pub mod master_node;
mod network;
mod utils;

pub async fn node_start(spawner: Spawner) {}
