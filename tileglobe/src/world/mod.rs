use core::ops::BitAnd;
use glam::{I16Vec2, I16Vec3, Vec3Swizzles};

pub mod block;
pub mod chunk;
pub mod utils;
pub mod world;

#[derive(
    Copy,
    Clone,
    derive_more::Deref,
    derive_more::DerefMut,
    derive_more::From,
    derive_more::Into,
    derive_more::Debug,
    derive_more::Display,
)]
#[debug("BlockPos({}, {}, {})", self.x, self.y, self.z)]
#[display("{}", self.0)]
pub struct BlockPos(I16Vec3);
impl BlockPos {
    pub fn chunk_pos(self) -> ChunkPos {
        ChunkPos(self.xz().div_euclid(I16Vec2::splat(16)))
    }

    pub fn chunk_local_pos(self) -> BlockPos {
        BlockPos(I16Vec3::from((self.xz().bitand(I16Vec2::splat(0xF)), self.y)).xzy())
    }
}

#[derive(
    Copy,
    Clone,
    derive_more::Deref,
    derive_more::DerefMut,
    derive_more::From,
    derive_more::Into,
    derive_more::Debug,
    derive_more::Display,
)]
#[debug("ChunkPos({}, {})", self.x, self.y)]
#[display("{}", self.0)]
pub struct ChunkPos(I16Vec2);