mod generic;
pub mod lever;
pub mod redstone_wire;
pub mod redstone_repeater;
pub mod redstone_comparator;
pub mod redstone_block;
mod redstone_torch;

pub use generic::*;
use tileglobe_utils::direction::Direction;
use tileglobe_utils::indexed_enum::IndexedEnum;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
#[repr(u8)]
enum AttachFace {
    FLOOR,
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
pub enum HorizontalDirection {
    NORTH,
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
impl TryFrom<Direction> for HorizontalDirection {
    type Error = ();

    fn try_from(value: Direction) -> Result<Self, Self::Error> {
        match value {
            Direction::NORTH => Ok(Self::NORTH),
            Direction::SOUTH => Ok(Self::SOUTH),
            Direction::WEST => Ok(Self::WEST),
            Direction::EAST => Ok(Self::EAST),
            _ => Err(()),
        }
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
    fn cw(self) -> HorizontalDirection {
        match self {
            HorizontalDirection::NORTH => HorizontalDirection::EAST,
            HorizontalDirection::EAST => HorizontalDirection::SOUTH,
            HorizontalDirection::SOUTH => HorizontalDirection::WEST,
            HorizontalDirection::WEST => HorizontalDirection::NORTH,
        }
    }
    fn ccw(self) -> HorizontalDirection {
        match self {
            HorizontalDirection::NORTH => HorizontalDirection::WEST,
            HorizontalDirection::WEST => HorizontalDirection::SOUTH,
            HorizontalDirection::SOUTH => HorizontalDirection::EAST,
            HorizontalDirection::EAST => HorizontalDirection::NORTH,
        }
    }
}