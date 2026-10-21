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
impl GpuResouce {
    pub fn mut_buffer(&self) -> &mut [u8] {
        match self {
            Self::Resource2D { address, len, .. } => unsafe {
                let ptr = *address as *mut u8;
                &mut *core::ptr::slice_from_raw_parts_mut(ptr, *len)
            },
            Self::Resource3D => todo!(),
        }
    }
}
