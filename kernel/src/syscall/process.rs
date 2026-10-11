use alloc::string::ToString;
use log::error;
use sync::mutex;

use crate::{
    sched::process::Process,
    syscall::{
        error::SyscallError,
        op::{SysOp, exit, str_from_args},
    },
    write_return,
};

/// Operations for creating, querying, and managing processes
pub enum ProcessOp {
    /// Returns the current PID
    CurrentPid = 0,
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
            Self::CurrentPid => {
                let pid = ktx.process()?.pid();
                write_return!(ktx, pid);
                Ok(())
            }
            Self::Spawn => {
                let (name_ptr, len, entry, ..) = args;
                let name = unsafe { str_from_args(name_ptr, len) }?;
                let entry_fn = entry as *const fn();
                match Process::spawn(name.as_str(), entry_fn) {
                    Ok(pid) => unsafe {
                        // exit 0 with value write
                        write_return!(ktx, pid);
                        Ok(())
                    },
                    Err(msg) => Err(SyscallError::Failed(msg.to_string())),
                }
            }
            Self::Fork => {
                let (name_ptr, name_len, entry, ..) = args;
                let name = unsafe { str_from_args(name_ptr, name_len)? };
                let entry_ptr = entry as *mut u32;
                let pid = ktx.process()?.fork(&name, entry_ptr)?;
                write_return!(ktx, pid);
                Ok(())
            }
            Self::Kill => {
                let (pid, ..) = args;
                let process = ktx.remove_process(pid as u32)?;
                process.destroy(); // deallocate process and drop asid + fd
                Ok(())
            }
            Self::Wait => {}
        }
    }
}

impl TryFrom<usize> for ProcessOp {
    type Error = SyscallError;
    fn try_from(value: usize) -> Result<Self, SyscallError> {
        Ok(match value {
            0 => Self::CurrentPid,
            1 => Self::Spawn,
            2 => Self::Fork,
            3 => Self::Wait,
            4 => Self::Kill,
            _ => return Err(SyscallError::InvalidOperation("ProcessOp", value)),
        })
    }
}
