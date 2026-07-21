use alloc::boxed::Box;
use log::warn;

use crate::sched::{
    queue::{RunQueue, THREAD_QUEUE},
    thread::Thread,
};

pub mod context;
pub mod queue;
pub mod thread;

/// intialize threading globals, setup main thread.
pub fn init_run_queue(stack_top: *const u8, stack_bottom: *const u8) {
    let queue = RunQueue::default();
    THREAD_QUEUE.init(|| queue);

    if let Err(error) = Thread::main(stack_top, stack_bottom) {
        warn!("Failed to setup main thread: {}", error);
    }
}
/// Manually terminates the current thread
#[macro_export]
macro_rules! terminate {
    () => {
        if let Some(queue) = unsafe { THREAD_QUEUE.get_mut() } {
            queue.terminate_running();
        }
    };
}
