use core::{ops::DerefMut, time::Duration, todo};

use alloc::format;

use crate::{
    drivers::gpu::GPU_DEVICE,
    syscall::{error::SyscallError, op::SysOp},
    write_return,
};

pub enum GfxOp {
    /// Returns information about the gpu modes and formats
    Info,
    /// Creates a 2d buffer with the given resource
    /// ## Args
    /// 1. `width: usize`
    /// 2. `height: usize`
    /// 3. `format: usize`
    CreateBuffer,
    /// Detatches, unmaps and frees the frames of the buffer with the given handle
    /// ## Args
    /// 1. `handle`: ``
    DestroyBuffer,
    /// Sets the display to the buffer with the given handle
    /// ## Args
    /// 1. `handle`
    Scanout,
    /// Makes the rect visible
    /// ## Args
    /// **None**
    Flush,
    /// Blocks the cpu until a flush or flip operation is complete.
    /// ## Args
    /// **None**
    Wait,
    /// Sets the cursor resource to a buffer with the specified id. Sets the cursor position to (x, y)
    SetCursor,
    /// Moves the cursor position by `dx`, `dy`
    /// ## Args
    /// **None**
    MoveCursor,
}

impl TryFrom<usize> for GfxOp {
    type Error = SyscallError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Info,
            1 => Self::CreateBuffer,
            2 => Self::DestroyBuffer,
            3 => Self::Scanout,
            4 => Self::Flush,
            5 => Self::Wait,
            6 => Self::SetCursor,
            7 => Self::MoveCursor,
            _ => return Err(SyscallError::InvalidOperation("GfxOp", value)),
        })
    }
}

struct GfxInfo {
    /// width of the display
    pub width: usize,
    /// height of the display
    pub height: usize,
    /// Refresh duration of the display
    pub refresh: Duration,
    /// Allowed color formats bitflags
    pub formats: u32,
}

impl SysOp for GfxOp {
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: super::SyscallArgs,
    ) -> Result<(), super::error::SyscallError> {
        match self {
            // GfxOp::CreateBuffer => {
            //     let process = ktx.process()?;
            //     process.address_space().map_pages(virt_addr, num_pages, flags, perms)
            // }
            // GfxOp::Info => {
            //     let display_driver = ktx.display_driver()?;
            //     display_driver.
            // }
            _ => todo!("Gfx Operations not yet implemented"),
        }
    }
}
