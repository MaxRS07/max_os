use core::{arch::asm, ops::DerefMut};

use abi::{
    call::Syscall,
    error::Errno,
    op::{ComOp, DevOp, FsOp, MemOp, ProcessOp, ThreadOp},
};
use alloc::string::ToString;
use log::{debug, error, warn};

use crate::{
    drivers::time::GLOBAL_TIME, fs::{GLOBAL_FS, call}, syscall::{
        context::{KernelContext, TrapFrame}, error::SyscallError, op::SysOp, trap_frame::TrapFrame,
    }, write_return,
};

mod com;
mod context;
mod device;
mod error;
mod fs;
mod mem;
mod op;
mod process;
mod thread;
mod trap_frame;

/// arguments 1-6. these come from the a0-5 registers.
type SyscallArgs = (usize, usize, usize, usize, usize, usize);

impl SysOp for Syscall {
    fn call(&self, ktx: &mut KernelContext, args: SyscallArgs) -> Result<(), SyscallError> {
        if let Err(err) = match self {
            Self::Device(op) => op.call(ktx, args),
            Self::Memory(op) => op.call(ktx, args),
            Self::FileSystem(op) => op.call(ktx, args),
            Self::Thread(op) => op.call(ktx, args),
            Self::Process(op) => op.call(ktx, args),
            Self::Communication(op) => op.call(ktx, args),
        } {
            // Propagate a0 write to here, write Syscall err pointer to following args
            log::error!("{err}");
            // leave syscall exit code in a0 for the user
            return Err(err);
        }
        Ok(())
    }
}

pub fn handle_ecall(frame: *mut usize) {
    let ktx = &mut KernelContext::new(frame);
    let mut id = 0usize;
    let args = match load_args(&mut id, ktx) {
        Ok(args) => args,
        Err(msg) => {
            error!("{msg}");
            return;
        }
    };
    // Adcance Supervisor Exception Program Counter (sepc) by 4 bytes, a full instruction.
    // otherwise sret will return to the ecall instruction and loop, pc is saved here on syscall
    let reg = unsafe { &mut *(frame as *mut TrapFrame) };
    reg.s[11] += 4;

    match Syscall::try_from(id) {
        Ok(op) => {
            match op.call(ktx, args) {
                Ok() => 
            }
        }
        Err(no) => {
            let code = no as isize;
            warn!("Call on {id} exited with code: {code}");
        }
    }
}
/// Loads the syscall registers from frame and returns them
pub fn load_args(id: &mut usize, ktx: &mut KernelContext) -> Result<SyscallArgs, SyscallError> {
    let frame = ktx.trap_frame();
    let [a0, a1, a2, a3, a4, a5, a6, a7, ..] = frame[5..13] else {
        return Err(SyscallError::InvalidArg(
            "Failed to load syscall arguments".to_string(),
        ));
    };
    *id = a7;
    // a6 unused to match riscv convention
    Ok((a0, a1, a2, a3, a4, a5))
}
