use crate::color::{Color, ColorFormat};

pub enum GpuResouce {
    /// 2d memory backed resource
    Resource2D {
        width: usize,
        height: usize,
        /// the length of the buffer in terms of objects
        len: usize,
        /// physical address of the buffer start
        address: usize,
        /// Color format
        format: ColorFormat,
    },
    Resource3D,
}
