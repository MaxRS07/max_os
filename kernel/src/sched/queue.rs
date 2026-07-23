use core::{
    fmt::Debug,
    ptr::{null, null_mut},
};

use log::{info, warn};
use sync::once::Once;

use crate::{
    sched::{
        context::switch_context_impl,
        thread::{State, Thread},
    },
    terminate,
};

/// Give thread 10k cycles (10ms) runtime before swapping
pub static THREAD_INTERVAL: usize = 10_000;

pub struct RunQueue {
    head: *mut Thread,
    tail: *mut Thread,
    running: *mut Thread,
    len: usize,
}

impl RunQueue {
    /// creates an empty `RunQueue`
    pub const fn new() -> Self {
        RunQueue {
            head: null_mut(),
            tail: null_mut(),
            len: 0,
            running: null_mut(),
        }
    }
    /// called on timer interrupt
    pub fn handle_interrupt(&mut self) {
        // if there is only the running, dont interrupt
        if self.head.is_null() {
            return;
        }
        self.run_next();
    }
    /// Inserts a thread in the queue, ordered by `thread.priority`
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // force safe. Not error handling this.
    pub fn enque(&mut self, thread: *mut Thread) -> Result<(), &'static str> {
        if thread.is_null() {
            return Err("Thread ptr is null");
        }

        unsafe {
            (*thread).next = null_mut();
            (*thread).prev = null_mut();

            // queue is empty
            if self.head.is_null() {
                self.head = thread;
                self.tail = thread;
                self.len = 1;
                if (*thread).state == State::Ready {
                    self.run_next();
                }
                return Ok(());
            }

            let mut current_ptr = self.head;

            while !current_ptr.is_null() {
                // Priority sorting
                if (*thread).priority < (*current_ptr).priority {
                    let previous_ptr = (*current_ptr).prev;

                    (*thread).next = current_ptr;
                    (*thread).prev = previous_ptr;
                    (*current_ptr).prev = thread;

                    if previous_ptr.is_null() {
                        self.head = thread;
                    } else {
                        (*previous_ptr).next = thread;
                    }
                    self.len += 1;
                    return Ok(());
                }

                // Node belongs at the very end of the list
                if (*current_ptr).next.is_null() {
                    (*current_ptr).next = thread;
                    (*thread).prev = current_ptr;
                    self.tail = thread; // Update tail
                    self.len += 1;
                    return Ok(());
                }

                current_ptr = (*current_ptr).next;
            }
        }
        Err("Failed to enqueue thread")
    }
    // Removes and returns the highest priority thread
    pub fn dequeue(&mut self) -> *mut Thread {
        if self.head.is_null() {
            return null_mut(); // Guard against empty queue
        }

        let head_node = self.head;
        unsafe {
            self.head = (*head_node).next;
            if self.head.is_null() {
                self.tail = null_mut(); // Queue became empty
            } else {
                (*self.head).prev = null_mut();
            }
            (*head_node).next = null_mut();
            (*head_node).prev = null_mut();
        }
        self.len -= 1;
        head_node
    }
    // Removes the pointed thread from the queue, returning it's pointee
    fn remove(&mut self, thread: *mut Thread) -> Result<&mut Thread, &'static str> {
        if thread.is_null() {
            return Err("Failed to dequeue thread: null");
        }
        unsafe {
            let prev_node = (*thread).prev;
            let next_node = (*thread).next;

            // A node with no links is only in the queue if it is the head
            // (i.e. the sole element). Otherwise it is detached — bailing out
            // here prevents corrupting head/tail and underflowing len.
            if prev_node.is_null() && next_node.is_null() && self.head != thread {
                return Err("Thread is not in the queue");
            }

            if prev_node.is_null() {
                self.head = next_node;
            } else {
                (*prev_node).next = next_node;
            }

            if next_node.is_null() {
                self.tail = prev_node; // update tail
            } else {
                (*next_node).prev = prev_node;
            }

            (*thread).next = null_mut();
            (*thread).prev = null_mut();
        }
        self.len -= 1;
        unsafe { Ok(&mut *thread) }
    }
    /// Adopts `thread` as the currently running thread without placing it in
    /// the ready queue. Used to bootstrap the initial (main) context so the
    /// first context switch has somewhere to save the boot registers.
    pub fn set_running(&mut self, thread: *mut Thread) {
        self.running = thread;
    }
    /// Starts the next thread and returns it's mutable pointer
    pub fn run_next(&mut self) -> *mut Thread {
        if !self.running.is_null() {
            unsafe {
                // if the thread is not terminated requeue it
                if (*self.running).state != State::Terminated {
                    (*self.running).state = State::Ready; // Mark the thread not running TODO: add completion detection
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

                    info!(
                        "Running {}, Queue: {:?}",
                        &*self.running,
                        QueueIterator { next: self.head }
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
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for RunQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for RunQueue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut binding = f.debug_struct("RunQueue");
        let w = binding.field("len", &self.len);

        if !self.running.is_null() {
            w.field("running", unsafe { &*self.running });
        }
        if !self.head.is_null() {
            w.field("head", unsafe { &*self.head });
        }
        w.field("queue", &QueueIterator { next: self.head });

        w.finish()
    }
}

unsafe impl Send for RunQueue {}
unsafe impl Sync for RunQueue {}

#[derive(Clone, Copy)]
struct QueueIterator {
    next: *mut Thread,
}

impl Iterator for QueueIterator {
    type Item = *mut Thread;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            return None;
        }
        unsafe {
            let cur = self.next;
            self.next = (*cur).next;
            Some(cur)
        }
    }
}
impl Debug for QueueIterator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("{ ")?;

        // Start from the current node without mutating 'self'
        let mut current = self.next;

        while !current.is_null() {
            unsafe {
                // Creat a safe temporary reference, NOT an owned value
                let thread_ref = &*current;

                // Print the thread using its own Debug implementation
                f.write_fmt(format_args!("{}, ", thread_ref))?;

                // Move to the next pointer in the raw linked list
                current = (*current).next;
            }
        }

        f.write_str("}")
    }
}
