use alloc::{boxed::Box, str::from_boxed_utf8_unchecked};
use log::{info, warn};
use sdt::fdt::FDT;
use sync::once::Once;

use crate::sched::{queue::THREAD_QUEUE, thread};
use core::{
    alloc::Layout,
    arch::naked_asm,
    cmp::min,
    fmt::Display,
    iter::Map,
    mem,
    ptr::{self, null_mut},
    sync::atomic::{AtomicU32, Ordering},
};

unsafe extern "C" {
    pub unsafe static _tdata_start: usize;
    pub unsafe static _tdata_end: usize;
    pub unsafe static _tbss_start: usize;
    pub unsafe static _tbss_end: usize;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Priority {
    /// Reserved for critical hardware drivers (e.g., UART, Disk Interrupts)
    RealTime = 0,
    /// High priority tasks (e.g., UI responsiveness or active kernel tasks)
    High = 1,
    /// The standard priority for everyday user applications
    #[default]
    Normal = 2,
    /// Background maintenance, logging, or cleaning
    Background = 3,
    /// lowest priority (system is idle)
    Idle = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Thread has entered, can be resumed on next run
    Ready,
    /// Currently running in the RunQueue.\
    /// `run_queue.running` always points to a thread in this state
    Running,
    /// Thread is blocked by task completion
    Waiting,
    /// Task completed, waiting for cleanup. Cannot be requeued.
    Terminated,
}
impl Default for State {
    /// Deafault state can be waiting, all threads are waiting on creation
    fn default() -> Self {
        Self::Waiting
    }
}
const THREAD_STACK_SIZE: usize = 0x3000;

#[derive(Debug, Clone, Default)]
pub struct Thread {
    /// information about the stack goes here. Contains both the stack pointer and thread pointer
    pub context: Context,
    /// Unique id of the thread
    id: u32,
    /// Thread name for debugging
    name: [u8; 16],
    /// Execution stage of the thread
    pub state: State,
    /// Relative execution priority of the thread, 0 is the highest priority
    pub priority: Priority,
    /// How long the thread has been alive in cpu cycles
    lifetime: u64,

    /// The total size of the thread's memory region
    total_size: usize,
    /// TLS start pointer. Also the absolute the start of the memory region.
    tls_start: *const u8,
    /// size of the thread's tls region in bytes
    tls_len: usize,

    /// Pointer to of the end of the stack. Points to the highest address in the stack (inclusive)
    stack_bottom: *const u8,
    /// Pointer to of the end of the stack. Points to the lowest address in the stack (inclusive)
    stack_top: *const u8,

    /// Pointer to next thread in priority queue
    pub next: *mut Thread,
    /// Pointer to previous thread in priority queue
    pub prev: *mut Thread,
}
/// records the last used thread id. IDs are assigned incrementally, every id after last is unused
static LAST_ID: AtomicU32 = AtomicU32::new(0);

impl Thread {
    /// Returns a preconfigured main thread. Does not have an entry point. Does not allocate stack or tls in memory.
    pub fn main(stack_top: *const u8, stack_bottom: *const u8) -> Result<(), &'static str> {
        let main = Self {
            id: 0,
            name: Self::name_from_str("main"),
            state: State::Running, // The main thread creates itself, hence it is running
            stack_top,
            stack_bottom,
            ..Self::default()
        };
        let main_box = Box::new(main);
        let main_ptr = Box::into_raw(main_box);

        unsafe {
            if let Some(thread_queue) = THREAD_QUEUE.get_mut() {
                // Main is the currently executing context, not a ready thread.
                // Register it as `running` so the first switch can save the
                // boot registers into it; enqueuing it would leave `running`
                // null forever and prevent any context switch.
                thread_queue.set_running(main_ptr);
                return Ok(());
            }
        }
        Err("Failed to queue main thread")
    }
    /// Creates and enques a thread. Returns the unique ID of the thread or None if queing failed.
    pub fn spawn(name: &str, priority: Priority, entry: usize) -> Option<u32> {
        let name = Self::name_from_str(name);
        let last_id = LAST_ID.load(Ordering::Acquire);
        let context = Context::empty();
        let mut new = Self {
            context,
            id: last_id + 1,
            name,
            state: State::Waiting,
            priority,
            lifetime: 0,
            ..Self::default()
        };
        if let Err(error) = new.alloc() {
            warn!("Failed to construct thread: {}", error);
            drop(new); // deallocate the memory on fail
            return None;
        }
        unsafe {
            let context = Context::new(entry, new.stack_bottom.addr(), new.tls_start.addr());
            new.context = context;
            new.state = State::Ready
        }

        let boxed = Box::new(new);
        let box_ptr = Box::into_raw(boxed);

        LAST_ID.store(last_id + 1, Ordering::Release);
        if let Some(thread_q) = unsafe { THREAD_QUEUE.get_mut() } {
            if thread_q.enque(box_ptr).is_err() {
                warn!(
                    "Failed to queue thread: '{}'",
                    str::from_utf8(&name).unwrap_or("")
                );
                let _ = unsafe { Box::from_raw(box_ptr) };
                return None;
            }
            return Some(last_id + 1);
        }
        let _ = unsafe { Box::from_raw(box_ptr) };
        None
    }
    /// Allocates memory and assigns region addresses to self
    fn alloc(&mut self) -> Result<(), &'static str> {
        // Calculte TSS size using pointers, then copy it to start pointer
        unsafe {
            // These are linker symbols: their *address* is the value we want, so
            // take `&raw const`. Reading them directly would load whatever word
            // lives at that address instead.
            let tdata_start = &raw const _tdata_start as usize;
            let tdata_end = &raw const _tdata_end as usize;
            let tbss_start = &raw const _tbss_start as usize;
            let tbss_end = &raw const _tbss_end as usize;

            let glob_tss_ptr = tdata_start as *const u8;
            let tbss_len = tbss_end - tbss_start;
            self.tls_len = tdata_end - tdata_start + tbss_len;

            let thread_layout =
                Layout::from_size_align(self.tls_len + THREAD_STACK_SIZE, 8).unwrap();
            let tls_mut = alloc::alloc::alloc(thread_layout).as_mut().unwrap();

            // copy data from global tls to local space
            core::ptr::copy_nonoverlapping(glob_tss_ptr, tls_mut, self.tls_len);
            self.tls_start = tls_mut;

            // set bounds for a 12KiB stack
            self.stack_top = self.tls_start.add(self.tls_len);
            self.stack_bottom = self.tls_start.add(self.tls_len).add(THREAD_STACK_SIZE);

            self.total_size = THREAD_STACK_SIZE + self.tls_len;
        }

        Ok(())
    }
    fn name_from_str(chars: &str) -> [u8; 16] {
        let mut name = [32u8; 16]; // Fill with white space
        let bytes = chars.as_bytes();
        let len = min(16, bytes.len());
        name[..len].copy_from_slice(&bytes[..len]);
        name
    }
    pub fn get_name_str(&self) -> &str {
        let name_bytes = self.name.trim_ascii();
        str::from_utf8(name_bytes).unwrap_or("?")
    }
    /// gets the mutable pointer
    pub fn mut_ptr(&mut self) -> *mut Thread {
        core::ptr::from_mut(self)
    }
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn is_main(&self) -> bool {
        self.id == 0
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.total_size, 8).unwrap();
        unsafe { alloc::alloc::dealloc(self.tls_start.cast_mut(), layout) };
    }
}

// forcing cross thread accessibility
unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}

impl Display for Thread {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{}@{:0x}",
            self.get_name_str(),
            self.tls_start.addr(),
        ))
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Thread context, contains information about
pub struct Context {
    /// saved pointer in the current stack, sp will be set here when this thread's context is loaded
    pub stack_pointer: *mut usize,
    /// pointer to the thread local storage block, tp is set to this when context is loaded
    pub thread_pointer: *mut usize,
}

impl Context {
    /// Prepares the stack for initial load. Returns the new Context
    /// # Safety
    pub unsafe fn new(entry: usize, stack_bottom: usize, tls_start: usize) -> Self {
        let frame_ptr = (stack_bottom - 56) as *mut u32;
        unsafe {
            // zero out all registers
            for i in 0..12 {
                frame_ptr.add(i).write(0);
            }
            // write the entry as an argument to the thread (a0)
            frame_ptr.write(entry as u32);
            // set tp register to our tls address
            frame_ptr.add(12).write(tls_start as u32);
            // set return to the function entry point
            frame_ptr
                .add(13)
                .write(thread_prolouge as extern "C" fn() as u32);
        }
        Context {
            thread_pointer: tls_start as *mut usize,
            stack_pointer: frame_ptr as *mut usize,
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
    naked_asm!("mv a0, s0", "tail thread_wrapper")
}
/// Wraps the thread entry in a terminating function, disposing the thread after execution
#[unsafe(no_mangle)]
extern "C" fn thread_wrapper(entry: u32) {
    unsafe {
        let func_ptr = entry as *const ();
        let func: fn() -> () = core::mem::transmute(func_ptr);
        func();
    }
    crate::terminate!();
}
