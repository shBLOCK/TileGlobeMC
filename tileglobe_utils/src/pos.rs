use crate::direction::Direction;
use glam::{I16Vec2, I16Vec3, Vec3Swizzles};

#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
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
    pub fn new(x: i16, y: i16, z: i16) -> Self {
        Self(I16Vec3::new(x, y, z))
    }

    pub fn chunk_pos(self) -> ChunkPos {
        ChunkPos(self.xz().div_euclid(I16Vec2::splat(16)))
    }

    pub fn chunk_local_pos(self) -> ChunkLocalPos {
        ChunkLocalPos::new(self.x as u8, self.y, self.z as u8)
    }

    pub fn offset_dir(self, direction: Direction) -> Self {
        Self(self.0 + direction.normal_i16())
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
impl ChunkPos {
    pub const fn new(x: i16, y: i16) -> ChunkPos {
        ChunkPos(I16Vec2::new(x, y))
    }
}

#[derive(
    Copy, Clone, derive_more::From, derive_more::Into, derive_more::Debug, derive_more::Display,
)]
#[debug("ChunkLocalPos({}, {}, {})", self.x(), self.y(), self.z())]
#[display("[{}, {}, {}]", self.x(), self.y(), self.z())]
pub struct ChunkLocalPos(u32);
impl ChunkLocalPos {
    pub fn new(x: u8, y: i16, z: u8) -> ChunkLocalPos {
        let (x, y, z) = (x as u32, y as u32, z as u32);
        ChunkLocalPos((x & 0xF) | ((z & 0xF) << 4) | (y << 8))
    }

    pub fn x(self) -> u8 {
        (self.0 & 0xF) as u8
    }
    pub fn z(self) -> u8 {
        ((self.0 >> 4) & 0xF) as u8
    }
    pub fn y(self) -> i16 {
        ((self.0 >> 8) & 0xFF_FF) as i16
    }
    pub fn section(self) -> i8 {
        self.y().div_euclid(16) as i8
    }
    pub fn section_block_index(self) -> u16 {
        (self.0 & 0xFFF) as u16
    }
}
