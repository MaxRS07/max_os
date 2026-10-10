use alloc::string::ToString;
use log::error;

use crate::{
    sched::process::Process,
    syscall::{
        error::SyscallError,
        op::{SysOp, exit, str_from_args},
    },
};

/// Operations for creating, querying, and managing processes
pub enum ProcessOp {
    /// Creates and registers a new blank process
    /// ## Args:
    /// 0. `name`: The pointer to the name str \
    /// 1. `name_len`: The length of the name in chars
    /// 2. ``
    Spawn = 1,
    /// Clones the current process into the process table, returning the clone's PID
    Fork = 2,
    /// Idk
    Wait = 3,
    /// Kills a process and terminates child threads
    Kill = 4,
}

impl SysOp for ProcessOp {
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: super::SyscallArgs,
    ) -> Result<(), SyscallError> {
        match self {
            Self::Spawn => {
                let (name_ptr, len, entry, pid_out, ..) = args;
                let name = unsafe { str_from_args(name_ptr, len) }?;
                let entry_fn = entry as *const fn();
                match Process::spawn(name.as_str(), entry_fn) {
                    Ok(pid) => unsafe {
                        // exit 0 with value write
                        (pid_out as *mut u32).write(pid);
                        Ok(())
                    },
                    Err(msg) => Err(SyscallError::Failed(msg.to_string())),
                }
            }
            Self::Fork => {}
            Self::Kill => {}
            Self::Wait => {}
        }
    }
}

impl TryFrom<usize> for ProcessOp {
    type Error = SyscallError;
    fn try_from(value: usize) -> Result<Self, SyscallError> {
        Ok(match value {
            0 => Self::Spawn,
            1 => Self::Fork,
            2 => Self::Wait,
            3 => Self::Kill,
            _ => return Err(SyscallError::InvalidOperation("ProcessOp", value)),
        })
    }
}
