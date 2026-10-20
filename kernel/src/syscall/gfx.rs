use core::todo;

use crate::{
    drivers::gpu::GPU_DEVICE,
    syscall::{error::SyscallError, op::SysOp},
};

pub enum GfxOp {
    /// Creates a gpu resource for writing output
    CreateResource,
    /// Destroy and deallocates a gpu resource
    DestroyResource,

    /// Writes a buffer to a specified resource
    WriteFrame,
    /// Locks the frame for a write
    LockFrame,
    /// Release the lock on this frame
    RelseaseFrame,

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
            2 => Self::WriteFrame,
            3 => Self::LockFrame,
            4 => Self::RelseaseFrame,
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
            GfxOp::CreateResource => todo!(),
            GfxOp::CreateResource => todo!(),
            GfxOp::DestroyResource => todo!(),
            GfxOp::WriteFrame => todo!(),
            GfxOp::LockFrame => todo!(),
            GfxOp::RelseaseFrame => todo!(),
            GfxOp::Flip => todo!(),
            GfxOp::Wait => todo!(),
            GfxOp::Info => todo!(),
        }
    }
}
