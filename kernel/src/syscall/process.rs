use crate::{
    sched::process::Process,
    syscall::{
        op::{SysOp, str_from_args},
        process::ProcessOp::Spawn,
    },
};

/// Operations for creating, querying, and managing processes
pub enum ProcessOp {
    /// Creates and registers a new process
    /// ## Args:
    /// 0. `name`: The pointer to the name str \
    /// 1. `name_len`: The length of the name in chars
    /// 2. ``
    Spawn,
}

impl SysOp for ProcessOp {
    fn call(&self, ktx: super::context::KernelContext, args: super::SyscallArgs) {
        match self {
            Spawn => {
                // let (name_ptr, len, entry, ..) = args;
                // let name = unsafe { str_from_args(name_ptr, len) };
                // if let SomeProcess::spawn(name, entry as *const fn())
                // core::arch::asm!("mv a0, {}", in(reg) )
            }
        }
    }
}
