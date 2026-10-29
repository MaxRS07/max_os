use core::{
    ptr::null_mut,
    sync::atomic::{AtomicPtr, AtomicU32, Ordering},
};

use alloc::{borrow::ToOwned, boxed::Box, collections::btree_map::IterMut, format};
use log::{debug, error, info, warn};
use mem::page_table::address_space::Addresser;
use sync::{mutex::Mutex, oncelock::OnceLock};

use crate::sched::{
    PROCESS_TABLE,
    context::{switch_address_space, thread_context_switch},
    error::ThreadError,
    process::{Process, ProcessTable},
    queue::RunQueue,
    thread::thread::{State, Thread, ThreadId},
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
    pub fn handle_interrupt(
        &mut self,
    ) -> Result<(*mut *mut usize, *mut usize, Option<usize>), ThreadError> {
        // if there is only the running, dont interrupt
        self.run_next()
    }
    /// Manually requeues the running thread and starts the next ready thread
    ///
    /// Unrestricted, works on kernel thread and main thread
    pub fn yield_thread(&mut self) {
        if let Ok((old, new, satp)) = self.run_next() {
            unsafe {
                // switch_context_impl(old, new, satp);
            }
        }
    }
    pub fn set_running(&mut self, id: ThreadId) -> Result<(), ThreadError> {
        let pid = id.pid;
        let tid = id.tid;
        let thread = PROCESS_TABLE
            .wait()
            .lock()
            .process_mut(pid)?
            .thread_mut(tid)
            .ok_or(ThreadError::Other(format!("Invalid TID on thread {pid}")))?;
        CURRENT_PROCESS.store(id.pid, Ordering::Release);
        self.running = Some(id);
        Ok(())
    }
    /// Interrupts the running thread and attempts to requeue it with the next available thread.
    ///
    /// If there is no next thread available to run, returns [`Err`] and does not interrupt the current thread. Returns [`Ok`] containing a pointer to the next available thread if the exchange was successful.
    pub fn run_next(
        &mut self,
    ) -> Result<(*mut *mut usize, *mut usize, Option<usize>), ThreadError> {
        // check next thread
        let Some(next_ready_id) = self.queue.dequeue_ready() else {
            return Err(ThreadError::Other(
                "No ready thread, failed to run next".to_owned(),
            ));
        };
        let mut process_table = PROCESS_TABLE.wait().lock();

        let next_ready = process_table.thread_mut(next_ready_id)?;

        if let Some(running_id) = self.running
            && let Ok(running) = PROCESS_TABLE.wait().lock().thread_mut(running_id)
        {
            let old_pid = running.pid();
            // if the thread is not terminated, requeue it. otherwise put it in the dead slot
            if running.state != State::Terminated {
                running.state = State::Ready;
                self.queue.enque(running_id)?
            } else {
                if let Some(dead_id) = self.dead {
                    // kill the zombie!
                    PROCESS_TABLE.wait().lock().drop_thread(dead_id);
                }
                self.dead = self.running;
            }

            next_ready.state = State::Running;

            let old_sp_ptr = &mut running.context.stack_pointer as *mut *mut usize;
            self.set_running(next_ready_id);

            let new_satp = if next_ready.pid() != old_pid {
                Some(
                    PROCESS_TABLE
                        .wait()
                        .lock()
                        .process(next_ready.pid())?
                        .address_space()
                        .satp(),
                )
            } else {
                None
            };

            // Switch context from old running thread to new
            let new_sp = (*next_ready).context.stack_pointer;

            return Ok((old_sp_ptr, new_sp, new_satp));
        }
        Err(ThreadError::Other(format!("What the fuck")))
    }
    /// Enqueues the thread with id `thread_id`, scheduling it to be run
    pub fn schedule(&mut self, id: ThreadId) -> Result<(), ThreadError> {
        self.queue.enque(id)
    }
    /// Removes and deallocates the thread with id `thread_id`. Cancelling the running thread is effectively identical to [`Self::run_next`] but discards [`Self::running`] instead of requeuing it
    pub fn canel_thread(&mut self, id: ThreadId) -> Result<(), ThreadError> {
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
    pub fn terminate_running(&mut self) -> Result<ThreadId, ThreadError> {
        match self.running {
            Some(running) => {
                let mut table = PROCESS_TABLE.wait().lock();
                let thread = table.thread_mut(running)?;
                thread.state = State::Terminated;
                Ok(running)
            }
            None => Err(ThreadError::Other(format!(
                "No thread running, failed to terminate"
            ))),
        }
    }
    // Pauses the current thread for `ms` milliseconds
    // pub fn sleep(&mut self, ms: u64) {} TODO

    /// Gets the pid of the current running thread. This is different from [`CURRENT_PROCESS`].
    pub fn current_pid(&self) -> Result<u32, ThreadError> {
        Ok(self
            .running
            .ok_or(ThreadError::Other("No current process".to_owned()))?
            .pid)
    }
}

unsafe impl Send for Scheduler {}
unsafe impl Sync for Scheduler {}
