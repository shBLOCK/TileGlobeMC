#[derive(
    Copy, Clone, derive_more::From, derive_more::Into, derive_more::Debug, derive_more::Display,
)]
#[debug("RGBA8({}, {}, {}, {})", self.r(), self.g(), self.b(), self.a())]
#[display("[{}, {}, {}, {}]", self.r(), self.g(), self.b(), self.a())]
pub struct RGBA8(u32);
impl RGBA8 {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> RGBA8 {
        let (r, g, b, a) = (r as u32, g as u32, b as u32, a as u32);
        RGBA8(r | (g << 8) | (b << 16) | (a << 24))
    }
    
    pub const fn r(self) -> u8 {
        self.0 as u8
    }
    pub const fn g(self) -> u8 {
        (self.0 >> 8) as u8
    }
    pub const fn b(self) -> u8 {
        (self.0 >> 16) as u8
    }
    pub const fn a(self) -> u8 {
        (self.0 >> 24) as u8
    }
}
