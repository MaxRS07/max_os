use crate::primitives::color::ColorFormat;

#[derive(Clone, Copy, Debug)]
pub struct SurfaceInfo {
    pub owner_id: u32,
    pub surface_id: u32,
}

#[derive(Clone, Debug)]
pub struct RenderCanvas {
    width: usize,
    height: usize,
    format: ColorFormat,
    stride: usize,
}

impl RenderCanvas {
    pub fn new(width: usize, height: usize, format: ColorFormat) -> Self {
        Self {
            width,
            height,
            format,
            stride: format.stride(width),
        }
    }
    /// The size of the buffer required to back this canvas in bytes
    pub fn buffer_size(&self) -> usize {
        self.width * self.height * self.format.pixel_bytes()
    }
}

pub type SurfaceId = u32;

/// The number of pages addresses needed for 480x640 resolution is `480 x 640 x 4 / 4096`
type StandardPageBuffer = [u64; 300];

#[derive(Clone, Debug)]
pub struct Surface {
    /* Ownership Info */
    pub info: SurfaceInfo,
    /* col and geo */
    pub canvas: RenderCanvas,
    /* backing */
    backing: SurfaceBacking,
}
impl Surface {
    pub fn new(info: SurfaceInfo, canvas: RenderCanvas, backing: SurfaceBacking) -> Self {
        Self {
            info,
            canvas,
            backing,
        }
    }
}
#[derive(Clone, Debug)]
pub struct SurfaceBacking {
    pub size: u64,
    pub page_count: usize,
    pub pages: StandardPageBuffer,
}

#[repr(C)]
pub struct SysSurfaceAllocRec {
    pub size_bytes: u64,
    pub flags: u32,
    pub out_surface_id: u32,
    pub out_gpu_vaddr: u32,
}
