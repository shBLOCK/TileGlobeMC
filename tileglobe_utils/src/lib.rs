#![no_std]

#![feature(generic_const_exprs)]

//TODO: now that tileglobe_proc_macro is gone, merge this into the tileglobe crate

extern crate alloc;

pub mod resloc;
pub mod color;
pub mod network;
pub mod indexed_enum;
pub mod pos;
pub mod direction;

pub const MINECRAFT: &str = "minecraft";
