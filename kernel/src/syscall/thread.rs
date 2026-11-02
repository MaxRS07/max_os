use core::arch::asm;

use alloc::{borrow::ToOwned, string::ToString};
use log::{error, warn};

use crate::{
    sched::{
        SCHEDULER,
        scheduler::Scheduler,
        thread::thread::{Priority, ThreadId},
    },
    syscall::{
        error::SyscallError,
        op::{SysOp, exit, str_from_args},
    },
    write_return,
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
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: super::SyscallArgs,
    ) -> Result<(), SyscallError> {
        match self {
            Self::Spawn => {
                let (pid, name, name_len, entry, ..) = args;
                let name_str = unsafe { str_from_args(name, name_len)? };
                let mut process = ktx.get_process(pid as u32)?;
                match process.spawn_thread(name_str.as_str(), entry) {
                    Some(id) => {
                        let tid = id.tid;
                        write_return!(ktx, pid as isize, tid as isize);
                        Ok(())
                    }
                    None => Err(SyscallError::Failed("Failed to queue thread".to_owned())),
                }
            }
            Self::Cancel => {
                let (pid, tid, ..) = args;
                let thread_id = ThreadId::new(pid as u32, tid as u32);
                SCHEDULER
                    .wait()
                    .lock()
                    .canel_thread(thread_id)
                    .map_err(|err| {
                        error!("{err}");
                        SyscallError::Failed(err.to_string())
                    })
            }
            Self::Yield => Ok(SCHEDULER.wait().lock().yield_thread()),
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
