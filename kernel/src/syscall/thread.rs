use log::error;

use crate::{
    sched::{
        SCHEDULER,
        scheduler::Scheduler,
        thread::{Priority, Thread},
    },
    syscall::op::{SysOp, str_from_args},
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
    fn call(&self, ktx: super::context::KernelContext, args: super::SyscallArgs) {
        match self {
            Self::Spawn => {
                let (pid, name, name_len, priority, entry, out_addr, ..) = args;
                let out_ptr = out_addr as *mut u32;
                let name_str = unsafe { str_from_args(name, name_len) };
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
                    unsafe { core::arch::asm!("li a0, -1") }
                }
            }
            Self::Cancel => {
                let (id, ..) = args;
                match SCHEDULER.wait().lock().canel_id(id as u32) {
                    Ok(_) => unsafe { core::arch::asm!("mv a0, x0") },
                    Err(msg) => unsafe {
                        core::arch::asm!("li a0, -1");
                        error!("{msg}")
                    },
                }
            }
            Self::Yield => SCHEDULER.wait().lock().yield_thread(),
        }
    }
}
