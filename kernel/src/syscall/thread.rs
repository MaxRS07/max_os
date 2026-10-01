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
                let (pid, name, name_len, priority, entry, ..) = args;
                let name_str = unsafe { str_from_args(name, name_len) };
                let id = Thread::spawn(pid as u32, name_str, Priority::from(priority), entry)?;
                unsafe { core::arch::asm!("mv a0, {}" in(reg) id) }
            }
            Self::Cancel => {
                let (id, ..) = args;
                SCHEDULER.wait().lock().canel_id(id as u32);
                unsafe { core::arch::asm!("mv a0, {}", in(reg) id) }
            }
            Self::Yield => SCHEDULER.wait().lock().yield_thread(),
        }
    }
}
