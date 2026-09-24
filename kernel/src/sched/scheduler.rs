use core::ptr::{null, null_mut};

use log::{debug, warn};

use crate::sched::{
    context::switch_context_impl,
    queue::RunQueue,
    thread::{State, Thread},
};

/// Ochestrates multiple thread queues, schedules CPU time per thread
pub struct Scheduler {
    running: *mut Thread,
    queue: RunQueue,
}

impl Scheduler {
    /// Manually pauses the running thread
    pub fn yield_thread() {}
    pub fn set_running(&mut self, thread: *mut Thread) {
        self.running = thread;
    }
    /// Starts the next thread and returns it's mutable pointer
    pub fn run_next(&mut self) -> *mut Thread {
        if !self.running.is_null() {
            unsafe {
                // if the thread is not terminated requeue it
                if (*self.running).state != State::Terminated {
                    (*self.running).state = State::Ready;
                    if let Err(error) = self.enque(self.running) {
                        warn!("Failed to reque active thread: {}", error);
                    }
                }
            }
        }
        let next = self.dequeue();
        if next.is_null() {
            return null_mut();
        }
        unsafe {
            if (*next).state == State::Ready {
                (*next).state = State::Running;
                if !self.running.is_null() {
                    // Switch context from old running thread to new
                    let old_sp_ptr = &mut (*self.running).context.stack_pointer as *mut *mut usize;
                    let new_sp = (*next).context.stack_pointer;

                    self.running = next;

                    debug!(
                        "Running {}, Queue: {:?}",
                        &*self.running,
                        self.queue.into_iter()
                    );

                    switch_context_impl(old_sp_ptr, new_sp);
                } else {
                    // There is no running thread, should not happen
                    return null_mut();
                }
            }
        }
        null_mut()
    }
    /// Removes and deallocates `Thread` pointed to by `thread`
    pub fn canel_thread(&mut self, thread: *mut Thread) -> Result<(), &'static str> {
        if self.remove(thread).is_ok() {
            return Ok(());
        }
        Err("Failed to canel thread")
    }
    /// Terminates the current thread, runs the next ready thread. It cannot terminate the main thread.
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
