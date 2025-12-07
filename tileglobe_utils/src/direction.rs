use glam::I16Vec3;
use crate::indexed_enum::IndexedEnum;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
#[repr(u8)]
pub enum Direction {
    DOWN,
    UP,
    NORTH,
    SOUTH,
    WEST,
    EAST,
}

impl IndexedEnum for Direction {
    type Index = u8;
    const VARIANTS: &'static [Self] = &[
        Self::DOWN,
        Self::UP,
        Self::NORTH,
        Self::SOUTH,
        Self::WEST,
        Self::EAST,
    ];
}

impl From<u8> for Direction {
    fn from(value: u8) -> Self {
        Self::variants()[value as usize]
    }
}

impl From<Direction> for u8 {
    fn from(value: Direction) -> Self {
        value as Self
    }
}

impl Direction {
    pub fn name(self) -> &'static str {
        ["down", "up", "north", "south", "west", "east"][self as usize]
    }

    pub fn normal_i16(self) -> I16Vec3 {
        [I16Vec3::NEG_Y, I16Vec3::Y, I16Vec3::NEG_Z, I16Vec3::Z, I16Vec3::NEG_X, I16Vec3::X][self as usize]
    }
    
    pub fn opposite(self) -> Direction {
        match self {
            Self::DOWN => Self::UP,
            Self::UP => Self::DOWN,
            Self::NORTH => Self::SOUTH,
            Self::SOUTH => Self::NORTH,
            Self::WEST => Self::EAST,
            Self::EAST => Self::WEST,
        }
    }
}

//TODO: cleanup using macros