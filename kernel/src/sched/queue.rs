use core::{
    fmt::Debug,
    ptr::{null, null_mut},
};

use alloc::collections::binary_heap::IntoIter;
use log::{info, warn};

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
    len: usize,
}

impl RunQueue {
    /// creates an empty `RunQueue`
    pub const fn new() -> Self {
        RunQueue {
            head: null_mut(),
            tail: null_mut(),
            len: 0,
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
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl IntoIterator for RunQueue {
    type IntoIter = QueueIterator;
    type Item = *mut Thread;
    fn into_iter(self) -> Self::IntoIter {
        QueueIterator { next: self.head }
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
                let thread_ref = &*current;
                f.write_fmt(format_args!("{}, ", thread_ref))?;
                // Move to the next pointer in the raw linked list
                current = (*current).next;
            }
        }

        f.write_str("}")
    }
}
