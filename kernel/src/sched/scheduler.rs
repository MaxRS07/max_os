use core::ptr::null_mut;

use alloc::{borrow::ToOwned, boxed::Box, collections::btree_map::IterMut, format};
use log::{debug, info, warn};
use mem::page_table::address_space::Addresser;
use sync::{mutex::Mutex, oncelock::OnceLock};

use crate::sched::{
    PROCESS_TABLE,
    context::switch_context_impl,
    error::ThreadError,
    process::Process,
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
    /// called on timer interrupt
    ///
    /// Dont call run next directly in case interrupt logic
    pub fn handle_interrupt(
        &mut self,
    ) -> Result<(*mut *mut usize, *mut usize, usize), ThreadError> {
        // if there is only the running, dont interrupt
        self.run_next()
    }
    /// Manually requeues the running thread and starts the next ready thread
    ///
    /// Unrestricted, works on kernel thread and main thread
    pub fn yield_thread(&self) {
        if let Ok((old, new, satp)) = self.run_next() {
            unsafe {
                switch_context_impl(old, new, satp);
            }
        }
    }
    pub fn set_running(&mut self, thread: *mut Thread) {
        self.running = thread;
    }
    /// Interrupts the running thread and attempts to requeue it with the next available thread.
    ///
    /// If there is no next thread available to run, returns [`Err`] and does not interrupt the current thread. Returns [`Ok`] containing a pointer to the next available thread if the exchange was successful.
    pub fn run_next(&mut self) -> Result<(*mut *mut usize, *mut usize, usize), ThreadError> {
        // check next thread
        let next_ready = self.queue.dequeue_ready();
        if next_ready.is_null() {
            return Err(ThreadError::Other(format!("No available threads to run")));
        }
        if !self.running.is_null() {
            unsafe {
                // if the thread is not terminated, requeue it
                if (*self.running).state != State::Terminated {
                    (*self.running).state = State::Ready;
                    self.queue.enque(self.running)?
                } else {
                    drop(Box::from_raw(self.running))
                }

                (*next_ready).state = State::Running;

                let old_sp_ptr = &mut (*self.running).context.stack_pointer as *mut *mut usize;
                self.set_running(next_ready);

                // Switch context from old running thread to new
                let new_sp = (*next_ready).context.stack_pointer;
                let new_satp = Self::get_process(next_ready)?.address_space().satp();

                return Ok((old_sp_ptr, new_sp, new_satp));
            }
        }
        Err(ThreadError::Other(format!("What the fuck")))
    }
    /// Enqueues `thread`, scheduling it to be run
    pub fn schedule(&mut self, thread: *mut Thread) -> Result<(), ThreadError> {
        Self::process_mut(thread)?.assign_thread(unsafe { (*thread).id() });
        self.queue.enque(thread)
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
            drop(Box::from_raw(self.running));
        }
        if let Ok((old, new, satp)) = self.run_next() {
            unsafe {
                switch_context_impl(old, new, satp);
            }
        }
    }
    pub fn canel_id(&mut self, id: u32) -> Result<(), ThreadError> {
        if unsafe { &*self.running }.id() == id {
            return self.canel_thread(self.running);
        }
        self.queue
            .remove_where(|thread| thread.id() == id)
            .ok_or(ThreadError::Other(
                "Failed to find thread with the specified id".to_owned(),
            ))
    }
    // Pauses the current thread for `ms` milliseconds
    // pub fn sleep(&mut self, ms: u64) {} TODO
    pub fn current_pid(&self) -> Result<u32, ThreadError> {
        if self.running.is_null() {
            return Err(ThreadError::Other(
                "Attempt to access null thread".to_owned(),
            ));
        }
        Ok((unsafe { &*self.running }).pid)
    }
    /// Returns the process that owns the current thread
    pub fn get_process<'a>(thread: *mut Thread) -> Result<&'a Process, ThreadError> {
        PROCESS_TABLE
            .wait()
            .get()
            .get(unsafe { &(*thread).pid })
            .map(|ptr| unsafe { &*ptr })
            .ok_or(ThreadError::Other(
                "Failed to retrive parent process, invalid PID".to_owned(),
            ))
    }
    /// Mutable ref to the process that owns the thread. This operation is locking.
    pub fn process_mut<'a>(thread: *mut Thread) -> Result<&'a mut Process, ThreadError> {
        PROCESS_TABLE
            .wait()
            .lock()
            .get(unsafe { &(*thread).pid })
            .map(|ptr| unsafe { &mut *ptr })
            .ok_or(ThreadError::Other(
                "Failed to retrive parent process, invalid PID".to_owned(),
            ))
    }
}

unsafe impl Send for Scheduler {}
unsafe impl Sync for Scheduler {}
