use log::error;

use crate::{
    sched::process::Process,
    syscall::op::{SysOp, str_from_args},
};

/// Operations for creating, querying, and managing processes
pub enum ProcessOp {
    /// Creates and registers a new blank process
    /// ## Args:
    /// 0. `name`: The pointer to the name str \
    /// 1. `name_len`: The length of the name in chars
    /// 2. ``
    Spawn,
    /// Clones the current process into the process table, returning the clone's PID
    Fork,
    /// Kills a process and terminates child threads
    Kill,
    /// Idk
    Wait,
}

impl SysOp for ProcessOp {
    fn call(&self, ktx: super::context::KernelContext, args: super::SyscallArgs) {
        match self {
            Self::Spawn => {
                let (name_ptr, len, entry, pid_out, ..) = args;
                let name = unsafe { str_from_args(name_ptr, len) };
                let entry_fn = entry as *const fn();
                match Process::spawn(name.as_str(), entry_fn) {
                    Ok(pid) => unsafe {
                        // exit 0 with value write
                        (pid_out as *mut u32).write(pid);
                        core::arch::asm!("mv a0, x0")
                    },
                    Err(msg) => unsafe {
                        error!("{msg}");
                        core::arch::asm!("li a0, -1")
                    },
                }
            }
            Self::Fork => {}
            Self::Kill => {}
            Self::Wait => {}
        }
    }
}
