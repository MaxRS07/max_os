use core::{
    cell::OnceCell,
    sync::atomic::{Ordering, fence},
};

use alloc::boxed::Box;
use log::{debug, info, warn};
use sync::once::OnceLock;

use crate::sched::{queue::RunQueue, thread::Thread};

pub mod context;
pub mod process;
pub mod queue;
pub mod thread;

pub static THREAD_QUEUE: OnceLock<RunQueue> = OnceLock::new();

/// intialize threading globals, setup main thread.
pub fn init_run_queue(stack_top: *const u8, stack_bottom: *const u8) {
    unsafe { THREAD_QUEUE.init(|| RunQueue::new()) };
    if let Err(error) = Thread::main(stack_top, stack_bottom) {
        warn!("Failed to setup main thread: {}", error);
    }
    info!("Created main thread");
}
/// Manually terminates the current thread
#[macro_export]
macro_rules! terminate {
    () => {
        unsafe {
            THREAD_QUEUE.get_mut().unwrap().terminate_running();
        }
    };
}

/// Manually pauses the
#[macro_export]
macro_rules! yield_thread {
    () => {
        unsafe {
            THREAD_QUEUE.get_mut().unwrap().terminate_running();
        }
    };
}
