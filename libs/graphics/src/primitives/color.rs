#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorFormat {
    #[default]
    Rgba,
    Bgra,
    Gbra,
}

impl ColorFormat {
    pub fn pixel_bytes(&self) -> usize {
        match self {
            Self::Rgba | Self::Bgra | Self::Gbra => 4,
        }
    }
    pub fn stride(&self, width: usize) -> usize {
        let unpadded_pitch = width * self.pixel_bytes();
        (unpadded_pitch + 63) & !63
    }
}
#[derive(Clone, Copy, Default, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(b: u8, g: u8, r: u8, a: u8) -> Self {
        Color { b, g, r, a }
    }
    /// Makes a color from bgra values 0..1
    pub fn from_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        let r = (r * 255f32) as u8;
        let g = (g * 255f32) as u8;
        let b = (b * 255f32) as u8;
        let a = (a * 255f32) as u8;
        Color { r, g, b, a }
    }
}
