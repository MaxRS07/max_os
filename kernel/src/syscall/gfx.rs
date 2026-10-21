use core::{ops::DerefMut, todo};

use alloc::format;

use crate::{
    drivers::gpu::GPU_DEVICE,
    syscall::{error::SyscallError, op::SysOp},
    write_return,
};

pub enum GfxOp {
    /// Creates a gpu resource for writing output
    /// ## Returns
    /// `a1`: The created resource's ID
    CreateResource,
    /// Destroy and deallocates a gpu resource
    /// ## Returns
    /// `a1`: Size of deallocated resource in bytes
    DestroyResource,

    /// Writes a buffer to a specified resource
    WriteResource,
    /// Locks the frame for a write
    LockResource,
    /// Release the lock on this frame
    ReleaseResource,

    /// Flips the current buffer, writing it to output
    Flip,
    ///
    Wait,
    Info,
}

impl TryFrom<usize> for GfxOp {
    type Error = SyscallError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::CreateResource,
            1 => Self::DestroyResource,
            2 => Self::WriteResource,
            3 => Self::LockResource,
            4 => Self::ReleaseResource,
            5 => Self::Flip,
            6 => Self::Wait,
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
            GfxOp::CreateResource => {
                let (width, height, format, ..) = args;
                let mut gpu = ktx.gpu()?;
                let resource_id = gpu
                    .create_resource(width, height)
                    .map_err(|err| SyscallError::Failed(format!("{err}")))?;
                write_return!(ktx, resource_id);
                Ok(())
            }
            GfxOp::DestroyResource => {
                let (resource_id, ..) = args;
                let mut gpu = ktx.gpu()?;
                let bytes = gpu.destroy_resource(resource_id as u32).map_err(|err| {
                    SyscallError::Failed(format!("Failed to destroy GPU resource: {err}"))
                })?;
                write_return!(ktx, bytes);
                Ok(())
            }
            GfxOp::WriteResource => {
                let (resource_id, buffer_addr, buffer_len, ..) = args;
                let gpu = ktx.gpu()?;
                let buffer_ptr = buffer_addr as *mut u8;
                let buffer = unsafe { &*core::ptr::slice_from_raw_parts(buffer_ptr, buffer_len) };
                let resource = gpu
                    .get_resource(resource_id as u32)
                    .ok_or(SyscallError::Failed(format!(
                        "Failed to write to resource {resource_id}, not found"
                    )))?;
                resource.mut_buffer().copy_from_slice(buffer);
                Ok(())
            }
            GfxOp::LockResource => todo!(),
            GfxOp::ReleaseResource => todo!(),
            GfxOp::Flip => todo!(),
            GfxOp::Wait => todo!(),
            GfxOp::Info => todo!(),
        }
    }
}
