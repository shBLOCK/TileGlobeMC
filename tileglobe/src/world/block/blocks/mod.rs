mod generic;
pub mod lever;
pub mod redstone_wire;
pub mod redstone_repeater;

pub use generic::*;
use tileglobe_utils::direction::Direction;
use tileglobe_utils::indexed_enum::IndexedEnum;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
#[repr(u8)]
enum AttachFace {
    FLOOR = 0,
    WALL,
    CEILING,
}
impl IndexedEnum for AttachFace {
    type Index = u8;
    const VARIANTS: &'static [Self] = &[
        Self::FLOOR,
        Self::WALL,
        Self::CEILING,
    ];
}
impl From<u8> for AttachFace {
    fn from(value: u8) -> Self {
        Self::variants()[value as usize]
    }
}
impl From<AttachFace> for u8 {
    fn from(value: AttachFace) -> Self {
        value as Self
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
#[repr(u8)]
enum HorizontalDirection {
    NORTH = 0,
    SOUTH,
    WEST,
    EAST,
}
impl IndexedEnum for HorizontalDirection {
    type Index = u8;
    const VARIANTS: &'static [Self] = &[
        Self::NORTH,
        Self::SOUTH,
        Self::WEST,
        Self::EAST,
    ];
}
impl From<u8> for HorizontalDirection {
    fn from(value: u8) -> Self {
        Self::variants()[value as usize]
    }
}
impl From<HorizontalDirection> for u8 {
    fn from(value: HorizontalDirection) -> Self {
        value as Self
    }
}
impl HorizontalDirection {
    fn direction(self) -> Direction {
        match self {
            HorizontalDirection::NORTH => Direction::NORTH,
            HorizontalDirection::EAST => Direction::EAST,
            HorizontalDirection::SOUTH => Direction::SOUTH,
            HorizontalDirection::WEST => Direction::WEST,
        }
    }
}