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
            let frame_ptr = (kstaktop as *mut usize).sub(13);
            let mut tp = *frame_ptr;
            let mut ra = *frame_ptr.add(4);
            let mut frame = (frame_ptr.add(8) as *mut [usize; 12]).read();

            match entry {
                Entry::Kernel { entry } => {
                    ra = thread_prolouge as usize;
                    frame[0] = entry
                }
                Entry::User { entry, user_sp } => {
                    // ra = user_thread_prolouge as usize;
                    frame[0] = entry;
                    frame[1] = user_sp;
                    frame[2] = kstaktop;
                }
            }

            (frame_ptr.add(8) as *mut &[usize]).write(&frame);
            frame_ptr.add(4).write(ra);
            frame_ptr.write(tp);

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
extern "C" fn thread_prolouge() {
    naked_asm!("csrsi sstatus, 2", "mv a0, s0", "tail thread_wrapper")
}
/// Wraps the thread entry in a terminating function, disposing the thread after execution.
///
/// Never returns: `ra` still points at [`thread_prolouge`], so returning would re-run the entry.
#[unsafe(no_mangle)]
extern "C" fn thread_wrapper(entry: u32) -> ! {
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
