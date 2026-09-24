use core::{
    cell::OnceCell,
    sync::atomic::{Ordering, fence},
};

use alloc::boxed::Box;
use log::{debug, info, warn};
use sync::oncelock::OnceLock;

use crate::sched::{scheduler::Scheduler, thread::Thread};

pub mod context;
pub mod error;
pub mod process;
pub mod queue;
pub mod scheduler;
pub mod thread;

/// This is the global thread scheduler
pub static SCHEDULER: OnceLock<Scheduler> = OnceLock::new();

/// intialize threading globals, setup main thread.
pub fn init_run_queue(stack_top: *const u8, stack_bottom: *const u8) {
    unsafe { SCHEDULER.init(|| Scheduler::new()) };
    if let Err(error) = Thread::main(stack_top, stack_bottom) {
        warn!("Failed to setup main thread: {}", error);
    }
    info!("Created main thread");
}
/// Manually terminates the current thread
///
/// Panics if the global scheduler has not been initialized
#[macro_export]
macro_rules! terminate {
    () => {
        unsafe {
            crate::sched::SCHEDULER
                .get_mut()
                .unwrap()
                .terminate_running();
        }
    };
}

/// Manually pauses the
#[macro_export]
macro_rules! yield_thread {
    () => {
        unsafe {
            crate::sched::SCHEDULER.get_mut().unwrap().yeild_thread();
        }
    };
}
