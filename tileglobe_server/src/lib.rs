#![no_std]
#![feature(generic_const_exprs)]

extern crate alloc;

pub mod mc_client;
pub mod mc_server;
pub mod player;
pub mod utils;

pub use mc_client::MCClient;
