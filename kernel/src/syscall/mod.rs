use core::arch::asm;

use crate::syscall::fs::FsOp;

mod com;
mod ctrl;
mod device;
mod fs;
mod mem;

/// top 8 bits for identifing the call type
const SYSCALL_MASK: usize = 0xFF << (usize::BITS - 8);
/// lower 24 for function routing
const FN_MASK: usize = !EXCEPTION_MASK;

/// arguments 1-6. these come from the a0-5 registers.
type SyscallArgs = (usize, usize, usize, usize, usize, usize);

enum Syscall {
    /// network
    Communication(),
    /// threadig
    Control,
    Device,
    /// File IO
    FileSystem(FsOp),
    /// allocation
    Memory,
    None,
}

fn route_call(value: SyscallArgs) -> Self {
    let call_type = value.0 & SYSCALL_MASK >> (usize::BITS - 8);
    let fn_type = 
    match call_type {
        1 => Self::Communication(com::Communication::),
        2 => Self::Control,
        3 => Self::Device,
        4 => Self::FileSystem,
        5 => Self::Memory(),
        _ => Self::None,
    } 
}

pub fn handle_ecall(call: u32) {
    let mut id: usize;
    let (arg1, ag2, arg3, arg4, arg5, arg6) = load_args(&mut id);
    match Syscall::from() {
        _ => {}
    }
}
/// Loads the syscall registers and returns them
pub fn load_args(id: &mut usize) -> SyscallArgs {
    let mut a7: usize; // syscall id + route
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
            out(reg) a7,
            out(reg) a0,
            out(reg) a1,
            out(reg) a2,
            out(reg) a3,
            out(reg) a4,
            out(reg) a5,
        )
    }
    (a7, a0, a1, a2, a3, a4, a5)
}
