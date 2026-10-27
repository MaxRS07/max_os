use core::fmt::Debug;

use alloc::{collections::VecDeque, string::ToOwned};

use crate::sched::{error::ThreadError, thread::thread::ThreadId};

pub static THREAD_INTERVAL: usize = 10_000;

#[derive(Debug)]
pub struct RunQueue {
    threads: VecDeque<ThreadId>,
}

impl RunQueue {
    pub const fn new() -> Self {
        Self {
            threads: VecDeque::new(),
        }
    }

    pub fn enque(&mut self, thread: ThreadId) -> Result<(), ThreadError> {
        if self.threads.contains(&thread) {
            return Err(ThreadError::Other(
                "Thread is already in the run queue".to_owned(),
            ));
        }

        self.threads.push_back(thread);
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<ThreadId> {
        self.threads.pop_front()
    }

    pub fn dequeue_ready(&mut self) -> Option<ThreadId> {
        self.dequeue()
    }

    pub fn remove(&mut self, thread: ThreadId) -> Result<ThreadId, ThreadError> {
        let index = self
            .threads
            .iter()
            .position(|queued| *queued == thread)
            .ok_or_else(|| ThreadError::Other("Thread is not in the run queue".to_owned()))?;

        self.threads
            .remove(index)
            .ok_or_else(|| ThreadError::Other("Failed to remove thread".to_owned()))
    }

    pub fn remove_first<F>(&mut self, mut predicate: F) -> Option<ThreadId>
    where
        F: FnMut(ThreadId) -> bool,
    {
        let index = self.threads.iter().position(|thread| predicate(*thread))?;

        self.threads.remove(index)
    }

    pub fn remove_where<F>(&mut self, mut predicate: F) -> usize
    where
        F: FnMut(ThreadId) -> bool,
    {
        let original_len = self.threads.len();
        self.threads.retain(|thread| !predicate(*thread));
        original_len - self.threads.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = ThreadId> + '_ {
        self.threads.iter().copied()
    }

    pub fn len(&self) -> usize {
        self.threads.len()
    }

    pub fn is_empty(&self) -> bool {
        self.threads.is_empty()
    }
}

impl Default for RunQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for RunQueue {
    type IntoIter = alloc::collections::vec_deque::IntoIter<ThreadId>;
    type Item = ThreadId;

    fn into_iter(self) -> Self::IntoIter {
        self.threads.into_iter()
    }
}

unsafe impl Send for RunQueue {}
unsafe impl Sync for RunQueue {}
