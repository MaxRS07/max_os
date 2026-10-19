use crate::{drivers::gpu::GPU_DEVICE, syscall::op::SysOp};

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
}

impl SysOp for GfxOp {
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: super::SyscallArgs,
    ) -> Result<(), super::error::SyscallError> {
        match self {
            GfxOp::CreateResource => {
                GPU_DEVICE.wait().lock().
            }
            GfxOp::CreateResource => todo!(),
            GfxOp::DestroyResource => todo!(),
            GfxOp::WriteFrame => todo!(),
            GfxOp::LockFrame => todo!(),
            GfxOp::RelseaseFrame => todo!(),
            GfxOp::Flip => todo!(),
            GfxOp::Wait => todo!(),
        }
    }
}
