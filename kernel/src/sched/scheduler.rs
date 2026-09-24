use core::ptr::null_mut;

use alloc::{borrow::ToOwned, format};
use log::{debug, warn};

use crate::sched::{
    context::switch_context_impl,
    error::ThreadError,
    queue::RunQueue,
    thread::{State, Thread},
};

/// Ochestrates multiple thread queues, schedules CPU time per process
pub struct Scheduler {
    running: *mut Thread,
    queue: RunQueue,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            running: null_mut(),
            queue: RunQueue::new(),
        }
    }
    /// Manually pauses the running thread
    pub fn yield_thread() {}
    pub fn set_running(&mut self, thread: *mut Thread) {
        self.running = thread;
    }
    /// Interrupts the running thread and attempts to requeue it with the next available thread.
    ///
    /// If there is no next thread available to run, returns [`Err`] and does not interrupt the current thread. Returns [`Ok`] containing a pointer to the next available thread if the exchange was successful.
    pub fn run_next(&mut self) -> Result<*mut Thread, ThreadError> {
        // check next thread
        let next_ready = self.queue.dequeue_ready();
        if next_ready.is_null() {
            return Err(ThreadError::Other(format!("No available threads to run")));
        }
        // TODO: This condition should always be true; make exchanges atomic
        if !self.running.is_null() {
            unsafe {
                // if the thread is not terminated, requeue it
                if (*self.running).state != State::Terminated {
                    (*self.running).state = State::Ready;
                    self.queue.enque(self.running)?
                }

                (*next_ready).state = State::Running;

                // Switch context from old running thread to new
                let old_sp_ptr = &mut (*self.running).context.stack_pointer as *mut *mut usize;
                let new_sp = (*next_ready).context.stack_pointer;

                self.running = next_ready;

                switch_context_impl(old_sp_ptr, new_sp);
            }
        }
        Ok(next_ready)
    }
    /// Removes and deallocates the `Thread` pointed to by `thread`. Cancelling the running thread is effectively identical to [`Self::run_next`] but discards [`Self::running`] instead of requeuing it
    pub fn canel_thread(&mut self, thread: *mut Thread) -> Result<(), ThreadError> {
        if self.running == thread {
            let next = self.queue.dequeue_ready();
            if !next.is_null() {
                self.set_running(next);
                return Ok(());
            }
        }
        if self.queue.remove(thread).is_ok() {
            return Ok(());
        }
        Err(ThreadError::Other("Failed to canel thread".to_owned()))
    }
    /// Terminates the current thread, runs the next ready thread.
    ///
    /// **NOTE:** Cannot terminate the main thread
    pub fn terminate_running(&mut self) {
        if self.running.is_null() {
            warn!("Attempted to terminate a null thread");
            return;
        }
        unsafe {
            if (*self.running).is_main() {
                warn!("Cannot terminate main thread");
                return;
            }
            (*self.running).state = State::Terminated;
        }
        self.run_next();
    }
    pub fn canel_id(&mut self, id: u32) -> Result<(), &'static str> {
        let mut current_ptr = self.head;
        while !current_ptr.is_null() {
            unsafe {
                if (*current_ptr).id() == id {
                    self.canel_thread(current_ptr)?
                }
                current_ptr = (*current_ptr).next;
            }
        }
        Err("Failed to find thread with the specified id")
    }
}

unsafe impl Send for Scheduler {}
unsafe impl Sync for Scheduler {}
