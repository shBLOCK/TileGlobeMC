#![no_std]

#![feature(generic_const_exprs)]

extern crate alloc;

pub mod network;
pub mod utils;
mod mc_client;
mod mc_server;
mod player;

pub use mc_client::MCClient;
