use core::arch::asm;

use log::{error, warn};

use crate::{
    sched::{
        SCHEDULER,
        scheduler::Scheduler,
        thread::{Priority, Thread},
    },
    syscall::{
        error::SyscallError,
        op::{SysOp, exit, str_from_args},
    },
};

/// Threading and process control system call operations
pub enum ThreadOp {
    /// Spawns a thread
    /// ## args:
    /// `pid`: Parent process id, \
    /// `name`: address of string pointer \
    /// `name_len`: length of name string \
    /// `priority`: runqueue priority of the thread \
    /// `entry`: address of entry function for this thread
    Spawn,
    // /// Causes the
    // Sleep,
    /// Forces the current thread to pause execution and requeue, giving CPU time to the next ready thread
    Yield,
    /// Cancels a thread, deallocating the used resources and space. If the specified thread is currently running the next ready thread will replace it
    Cancel,
}

impl SysOp for ThreadOp {
    fn call(&self, ktx: &mut super::context::KernelContext, args: super::SyscallArgs) {
        match self {
            Self::Spawn => {
                let (pid, name, name_len, priority, entry, out_addr, ..) = args;
                let out_ptr = out_addr as *mut u32;
                let name_str = unsafe {
                    match str_from_args(name, name_len) {
                        Ok(value) => value,
                        Err(err) => {
                            exit(ktx, -1);
                            error!("{err}");
                            return;
                        }
                    }
                };
                if let Some(id) = Thread::spawn(
                    pid as u32,
                    name_str.as_str(),
                    Priority::from(priority),
                    entry,
                ) {
                    // exit ok (0)
                    unsafe {
                        core::arch::asm!("mv a0, x0");
                        core::ptr::write(out_ptr, id);
                    }
                } else {
                    // Exit with error (-1)
                    exit(ktx, -1);
                }
            }
            Self::Cancel => {
                let (id, ..) = args;
                match SCHEDULER.wait().lock().canel_id(id as u32) {
                    Ok(_) => exit(ktx, 0),
                    Err(msg) => {
                        exit(ktx, -1);
                        error!("{msg}")
                    }
                }
            }
            Self::Yield => SCHEDULER.wait().lock().yield_thread(),
        }
    }
}

impl TryFrom<usize> for ThreadOp {
    type Error = SyscallError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Spawn,
            1 => Self::Yield,
            2 => Self::Cancel,
            _ => return Err(SyscallError::InvalidOperation("ThreadOp", value)),
        })
    }
}
