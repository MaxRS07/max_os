use core::{
    arch::naked_asm,
    ptr::null_mut,
    slice::{from_raw_parts, from_raw_parts_mut},
};

use log::error;

use crate::sched::{SCHEDULER, swap::switch_to};

pub enum Entry {
    Kernel { entry: usize },
    User { entry: usize, user_sp: usize },
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Thread context, contains information about the execution state of the thread
pub struct Context {
    /// saved pointer in the current stack, sp will be set here when this thread's context is loaded
    pub stack_pointer: *mut usize,
    /// pointer to the thread local storage block, tp is set to this when context is loaded
    pub thread_pointer: *mut usize,
}
impl Context {
    /// Prepares the stack for initial load. Returns the new Context
    /// # Safety
    pub fn new(entry: Entry, kstaktop: usize, tls_start: usize) -> Self {
        unsafe {
            let frame_ptr = (kstaktop as *mut usize).sub(14);
            let frame = &mut *(frame_ptr as *mut [usize; 14]);

            // zero out registers
            frame.copy_from_slice(&[0; 14]);

            match entry {
                Entry::Kernel { entry } => {
                    frame[13] = kernel_thread_prologue as *const fn() as usize;
                    frame[0] = entry
                }
                Entry::User { entry, user_sp } => {
                    frame[0] = entry;
                    frame[1] = user_sp;
                    frame[2] = kstaktop;
                    frame[13] = user_thread_prologue as *const () as usize;
                }
            }

            Context {
                thread_pointer: tls_start as *mut usize,
                stack_pointer: frame_ptr as *mut usize,
            }
        }
    }
    /// creates a new `Context` with null pointers
    pub fn empty() -> Self {
        Self {
            thread_pointer: null_mut(),
            stack_pointer: null_mut(),
        }
    }
}

impl Default for Context {
    /// Defaults to empty thread. See `Context::empty()`.
    fn default() -> Self {
        Self::empty()
    }
}
/// Loads the entry function into the first arg of the thread wrapper
#[unsafe(naked)]
extern "C" fn kernel_thread_prologue() {
    naked_asm!(
        "csrsi sstatus, 2",
        "mv a0, s0",
        "tail kernel_thread_wrapper"
    )
}
/// Wraps the thread entry in a terminating function, disposing the thread after execution.
///
/// Never returns: `ra` still points at [`kernel_thread_prologue`], so returning would re-run the entry.
#[unsafe(no_mangle)]
extern "C" fn kernel_thread_wrapper(entry: usize) -> ! {
    unsafe {
        let func_ptr = entry as *const ();
        let func: fn() -> () = core::mem::transmute(func_ptr);
        func();

        // stop the timer from switching between terminating and switching
        core::arch::asm!("csrci sstatus, 2");
        // release the scheduler lock before switching, this stack is never resumed
        let next = {
            let mut sched = SCHEDULER.wait().lock();
            let _ = sched.terminate_running();
            sched.run_next()
        };
        match next {
            Ok(context) => switch_to(context),
            Err(err) => error!("Thread exit: {err}"),
        }
        loop {
            core::arch::asm!("wfi");
        }
    }
}

#[unsafe(naked)]
extern "C" fn user_thread_prologue() {
    naked_asm!(
        "csrci sstatus, 2",
        "csrw sepc, s0",
        // user bits
        "li t0, (1 << 8)",
        "csrc sstatus, t0",
        "li t0, (1 << 5)",
        "csrs sstatus, t0",
        // write sscratch
        "csrw sscratch, s2",
        "mv sp, s1",
        // zero frame to prevent kernel registers leaking
        "mv a0, x0",
        "mv a1, x0",
        "mv a2, x0",
        "mv a3, x0",
        "mv a4, x0",
        "mv a5, x0",
        "mv a6, x0",
        "mv a7, x0",
        "mv t0, x0",
        "mv t1, x0",
        "mv t2, x0",
        "mv t3, x0",
        "mv t4, x0",
        "mv t5, x0",
        "mv t6, x0",
        "mv s0, x0",
        "mv s1, x0",
        "mv s2, x0",
        "mv s3, x0",
        "mv s4, x0",
        "mv s5, x0",
        "mv s6, x0",
        "mv s7, x0",
        "mv s8, x0",
        "mv s9, x0",
        "mv s10, x0",
        "mv s11, x0",
        // s2 contains the top of the kernel stack
        "mv tp, zero",
        "sret",
    )
}
