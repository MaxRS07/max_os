use core::arch::asm;

use log::warn;

use crate::{
    fs::GLOBAL_FS,
    syscall::{com::ComOp, context::KernelContext, error::SyscallError, fs::FsOp, op::SysOp},
};

mod com;
mod context;
mod ctrl;
mod device;
mod error;
mod fs;
mod mem;
mod op;
/// top 8 bits for identifing the call type
const SYSCALL_MASK: usize = 0xFF << (usize::BITS - 8);
/// lower 24 for function routing
const FN_MASK: usize = !SYSCALL_MASK;

/// arguments 1-6. these come from the a0-5 registers.
type SyscallArgs = (usize, usize, usize, usize, usize, usize);

enum Syscall {
    /// network
    Communication(ComOp),
    /// threadig
    Control,
    Device,
    /// File IO
    FileSystem(FsOp),
    /// allocation
    Memory,
}

impl SysOp for Syscall {
    fn call(&mut self, ktx: KernelContext, args: SyscallArgs) {
        match self {
            _ => (),
        }
    }
}
impl Syscall {
    pub fn from_call_fn(call_type: usize, fn_type: usize) -> Result<Self, SyscallError> {
        Ok(match call_type {
            1 => Syscall::Communication(ComOp::from(fn_type)),
            2 => Syscall::Control,
            3 => Syscall::Device,
            4 => Syscall::FileSystem(FsOp::from(fn_type)),
            5 => Syscall::Memory,
            _ => return Err(SyscallError::InvalidCall),
        })
    }
}

fn route_call(value: usize) -> Result<Syscall, SyscallError> {
    let call_type = value & SYSCALL_MASK >> (usize::BITS - 8);
    let fn_type = value & FN_MASK;
    Syscall::from_call_fn(call_type, fn_type)
}

pub fn handle_ecall() {
    let Some(fs) = GLOBAL_FS.get_mut() else {
        warn!("Failed to get GLOBAL_FS instance");
        return;
    };
    let mut ktx = KernelContext::new(fs);
    let mut id = 0usize;
    let args = load_args(&mut id);
    match route_call(id) {
        Ok(op) => op.call(ktx, args),
        Err(msg) => warn!(msg),
    }
}
/// Loads the syscall registers and returns them
pub fn load_args(id: &mut usize) -> SyscallArgs {
    let mut _id: usize; // syscall id + route
    let mut a0: usize; // arg 1
    let mut a1: usize; // arg 2
    let mut a2: usize; // arg 3
    let mut a3: usize; // arg 4
    let mut a4: usize; // arg 5
    let mut a5: usize; // arg 6

    unsafe {
        asm!(
            "mv {}, a7",
            "mv {}, a0",
            "mv {}, a1",
            "mv {}, a2",
            "mv {}, a3",
            "mv {}, a4",
            "mv {}, a5",
            out(reg) _id,
            out(reg) a0,
            out(reg) a1,
            out(reg) a2,
            out(reg) a3,
            out(reg) a4,
            out(reg) a5,
        )
    }
    *id = _id;
    (a0, a1, a2, a3, a4, a5)
}
