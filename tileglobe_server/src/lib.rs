#![no_std]

#![feature(generic_const_exprs)]

extern crate alloc;

pub mod utils;
pub mod mc_client;
pub mod mc_server;
pub mod player;

pub use mc_client::MCClient;
