// libs/graphics/src/text/font.rs

use crate::text::font_data::{FontData, PC_FACE_MODERNDOS_8X16_FONT_LIST};

pub const FONT_WIDTH: usize = 8;
pub const FONT_HEIGHT: usize = 16;

pub enum SystemFont8X16 {
    Modernos,
}

impl SystemFont8X16 {
    pub fn data(&self) -> &'static FontData {
        match self {
            SystemFont8X16::Modernos => &PC_FACE_MODERNDOS_8X16_FONT_LIST,
        }
    }
    pub fn glyph(&self, c: u8) -> &'static [u8; 16] {
        &self.data()[c as usize]
    }
}
