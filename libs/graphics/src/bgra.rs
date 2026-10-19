#[derive(Clone, Copy, Default, Debug)]
pub struct Brga {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

impl Brga {
    pub fn new(b: u8, g: u8, r: u8, a: u8) -> Self {
        Brga { b, g, r, a }
    }
    /// Makes a color from bgra values 0..1
    pub fn from_f32(b: f32, g: f32, r: f32, a: f32) -> Self {
        let b = (b * 255f32) as u8;
        let g = (g * 255f32) as u8;
        let r = (r * 255f32) as u8;
        let a = (a * 255f32) as u8;
        Brga { b, g, r, a }
    }
}
