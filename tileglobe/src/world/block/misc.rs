use tileglobe_utils::color::RGBA8;

//TODO: paletted / id based?
#[derive(Debug)]
pub struct MapColor {
    color: RGBA8,
}
impl MapColor {
    pub const fn new(color: RGBA8) -> MapColor {
        MapColor { color }
    }

    pub const fn color(&self) -> RGBA8 {
        self.color
    }
}
