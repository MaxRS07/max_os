use core::{ops::DerefMut, todo};

use alloc::format;
use graphics::color::ColorFormat;

use crate::{
    drivers::gpu::GPU_DEVICE,
    syscall::{error::SyscallError, op::SysOp},
    write_return,
};

pub enum GfxOp {
    /// Creates a writable surface
    /// ## Returns
    /// `a1`: The created resource's ID
    SurfaceAlloc,
    /// Destroy and deallocates a gpu resource
    /// ## Returns
    /// `a1`: Size of deallocated resource in bytes
    SurfaceDealloc,
    Info,
}

impl TryFrom<usize> for GfxOp {
    type Error = SyscallError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::SurfaceAlloc,
            1 => Self::SurfaceDealloc,
            7 => Self::Info,
            _ => return Err(SyscallError::InvalidOperation("GfxOp", value)),
        })
    }
}

impl SysOp for GfxOp {
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: super::SyscallArgs,
    ) -> Result<(), super::error::SyscallError> {
        match self {
            GfxOp::SurfaceAlloc => {
                let (width, height, format, ..) = args;
                let mut gpu = ktx.gpu()?;
                let resource_id = gpu
                    .create_surface(width, height, ColorFormat::Rgba)
                    .map_err(|err| SyscallError::Failed(format!("{err}")))?;
                write_return!(ktx, resource_id);
                Ok(())
            }
            GfxOp::SurfaceDealloc => {
                let (resource_id, ..) = args;
                let mut gpu = ktx.gpu()?;
                let bytes = gpu.destroy_surface(resource_id as u32).map_err(|err| {
                    SyscallError::Failed(format!("Failed to destroy GPU resource: {err}"))
                })?;
                write_return!(ktx, bytes);
                Ok(())
            }
            GfxOp::Info => todo!(),
        }
    }
}
