use core::arch::asm;

use abi::op::ThreadOp;
use alloc::{borrow::ToOwned, format, string::ToString};
use log::{error, warn};

use crate::{
    sched::{
        self, SCHEDULER,
        scheduler::Scheduler,
        spawn_thread,
        thread::thread::{Priority, ThreadId},
    },
    syscall::{
        error::SyscallError,
        op::{SysOp, exit, str_from_args},
    },
    write_return, yield_thread,
};

impl SysOp for ThreadOp {
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: super::SyscallArgs,
    ) -> Result<(), SyscallError> {
        match self {
            Self::Spawn => {
                let (name, name_len, entry, ..) = args;
                let name_str = unsafe { str_from_args(name, name_len)? };
                let pid = ktx.process()?.pid();
                match spawn_thread(pid as u32, name_str.as_str(), entry) {
                    Ok(id) => {
                        let tid = id.tid;
                        write_return!(ktx, pid as isize, tid as isize);
                        Ok(())
                    }
                    Err(err) => Err(SyscallError::Failed(format!(
                        "Failed to queue thread: {err}"
                    ))),
                }
            }
            Self::Exit => sched::exit_current_thread(),
            Self::Cancel => {
                let (pid, tid, ..) = args;
                let thread_id = ThreadId::new(pid as u32, tid as u32);
                SCHEDULER
                    .wait()
                    .lock()
                    .remove_thread(thread_id)
                    .ok_or(SyscallError::Failed(format!(
                        "Failed to remove thread with {thread_id:?}, does not exist"
                    )))?;
                Ok(())
            }
            Self::Yield => Ok(yield_thread()),
        }
    }
}
