#![no_std]

#![feature(generic_const_exprs)]

extern crate alloc;

pub mod resloc;
pub mod color;
pub mod network;
pub mod indexed_enum;
pub mod pos;
pub mod direction;

pub const MINECRAFT: &str = "minecraft";
