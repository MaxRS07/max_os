use core::{
    ptr::null_mut,
    sync::atomic::{AtomicPtr, AtomicU32, Ordering},
};

use alloc::{borrow::ToOwned, format};
use fs::meta::context;
use log::{debug, error, info, warn};
use mem::page_table::address_space::Addresser;
use sync::{mutex::Mutex, oncelock::OnceLock};

use crate::sched::{
    PROCESS_TABLE,
    context::ThreadContextSwitch,
    error::SchedError,
    queue::RunQueue,
    thread::thread::{State, ThreadId},
};

pub static CURRENT_PROCESS: AtomicU32 = AtomicU32::new(0);

/// Ochestrates multiple thread queues, schedules CPU time per process
pub struct Scheduler {
    running: Option<ThreadId>,
    /// The dead thread should still exist on its parent process
    dead: Option<ThreadId>,
    queue: RunQueue,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            running: None,
            dead: None,
            queue: RunQueue::new(),
        }
    }
    /// called on timer interrupt
    ///
    /// Dont call run next directly in case interrupt logic
    pub fn handle_interrupt(&mut self) -> Result<ThreadContextSwitch, SchedError> {
        // if there is only the running, dont interrupt
        self.run_next()
    }
    pub fn set_running(&mut self, id: ThreadId) -> Result<(), SchedError> {
        // make sure thread exists
        if !PROCESS_TABLE
            .wait()
            .lock()
            .process_mut(id.pid)?
            .contains_thread(id.tid)
        {
            return Err(SchedError::BadTid(id));
        }
        CURRENT_PROCESS.store(id.pid, Ordering::Release);
        self.running = Some(id);
        Ok(())
    }
    /// Interrupts the running thread and attempts to requeue it with the next available thread.
    ///
    /// If there is no next thread available to run, returns [`Err`] and does not interrupt the current thread. Returns [`Ok`] containing a pointer to the next available thread if the exchange was successful.
    pub fn run_next(&mut self) -> Result<ThreadContextSwitch, SchedError> {
        // check next thread
        let Some(next_ready_id) = self.queue.dequeue_ready() else {
            return Err(SchedError::Other(
                "No ready thread, failed to run next".to_owned(),
            ));
        };

        let Some(running_id) = self.running else {
            let _ = self.queue.enque(next_ready_id);
            return Err(SchedError::Other(
                "No running thread, failed to swap context".to_owned(),
            ));
        };

        // usages of ts need to be in blocks to guarentee the guards are dropped after use
        let mut table = PROCESS_TABLE.wait().lock();

        let check = (|| {
            let new_satp = if next_ready_id.pid != running_id.pid {
                Some(table.process(next_ready_id.pid)?.address_space().satp())
            } else {
                None
            };
            let done = {
                let running = table.thread_mut(running_id)?;
                running.state == State::Terminated
            };
            // check that the next thread exists, ref not needed
            let next = table.thread_mut(next_ready_id)?;
            debug!("switching from thread {running_id:?} to {next}");
            Ok((new_satp, done))
        })();

        let (new_satp, teminated) = match check {
            Ok(v) => v,
            Err(err) => {
                self.queue.enque(next_ready_id)?;
                return Err(err);
            }
        };

        if teminated {
            if let Some(dead_thread) = self.dead.replace(running_id) {
                if let Err(err) = table.drop_thread(dead_thread) {
                    warn!("Failed to free zombie thread {dead_thread:?}: {err}")
                }
            } else {
                self.queue.enque(running_id)?;
            }
        } else {
            table.thread_mut(running_id)?.state = State::Ready;
            self.queue.enque(running_id)?;
        }

        let new_sp = {
            let next_ready = table.thread_mut(next_ready_id)?;
            next_ready.state = State::Running;
            next_ready.context.stack_pointer
        };
        let old_sp_ptr = {
            let running = table.thread_mut(running_id)?;
            &raw mut running.context.stack_pointer
        };

        // set_running would relock the table, the thread was already checked above
        CURRENT_PROCESS.store(next_ready_id.pid, Ordering::Release);
        self.running = Some(next_ready_id);

        Ok(ThreadContextSwitch {
            new_satp,
            new_sp,
            old_sp_ptr,
        })
    }
    /// Enqueues the thread with id `thread_id`, scheduling it to be run
    pub fn schedule(&mut self, id: ThreadId) -> Result<(), SchedError> {
        self.queue.enque(id)
    }
    /// Removes and deallocates the thread with id `thread_id`. Cancelling the running thread is effectively identical to [`Self::run_next`] but discards [`Self::running`] instead of requeuing it
    pub fn canel_thread(&mut self, id: ThreadId) -> Result<(), SchedError> {
        let mut process_table = PROCESS_TABLE.wait().lock();
        if self.running.is_some_and(|t| t.tid == id.tid) {
            self.running = None;
        }
        self.queue.remove(id);
        process_table.process_mut(id.pid)?.remove_thread(id.tid);
        Ok(())
    }
    /// Terminates the current thread, runs the next ready thread.
    ///
    /// **NOTE:** Cannot terminate the main thread
    pub fn terminate_running(&mut self) -> Result<ThreadId, SchedError> {
        match self.running {
            Some(running) => {
                let mut table = PROCESS_TABLE.wait().lock();
                let thread = table.thread_mut(running)?;
                thread.state = State::Terminated;
                Ok(running)
            }
            None => Err(SchedError::Other(format!(
                "No thread running, failed to terminate"
            ))),
        }
    }
    // Pauses the current thread for `ms` milliseconds
    // pub fn sleep(&mut self, ms: u64) {} TODO

    /// Gets the pid of the current running thread. This is different from [`CURRENT_PROCESS`].
    pub fn current_pid(&self) -> Result<u32, SchedError> {
        Ok(self
            .running
            .ok_or(SchedError::Other("No current process".to_owned()))?
            .pid)
    }
}

unsafe impl Send for Scheduler {}
unsafe impl Sync for Scheduler {}
