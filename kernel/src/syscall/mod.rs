use core::arch::asm;

use crate::syscall::{com::ComOp, fs::FsOp};

mod com;
mod ctrl;
mod device;
mod fs;
mod mem;

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

fn route_call(value: usize) -> Syscall {
    let call_type = value & SYSCALL_MASK >> (usize::BITS - 8);
    let fn_type = value & FN_MASK;
    match call_type {
        1 => Syscall::Communication(ComOp::from(fn_type)),
        2 => Syscall::Control,
        3 => Syscall::Device,
        4 => Syscall::FileSystem(FsOp::from(fn_type)),
        5 => Syscall::Memory,
        _ => panic!("Unsupported syscall"),
    }
}

pub fn handle_ecall() {
    let mut id = 0usize;
    let (arg1, ag2, arg3, arg4, arg5, arg6) = load_args(&mut id);
    match route_call(id) {
        _ => {}
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
