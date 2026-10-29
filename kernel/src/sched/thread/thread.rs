use ::mem::page_table::{address_space::Addresser, permissions::PagePermissions};
use alloc::{
    borrow::ToOwned,
    boxed::Box,
    format,
    str::from_boxed_utf8_unchecked,
    string::{String, ToString},
    vec::Vec,
};
use fs::meta::context;
use log::{debug, info, warn};
use sdt::fdt::FDT;

use crate::{
    console::writer::println,
    sched::{
        PROCESS_TABLE, SCHEDULER,
        context::switch_address_space,
        error::ThreadError,
        process::ProcessTable,
        scheduler::{CURRENT_PROCESS, switch_to},
        thread::{self, kstack::KernelStack, ustack::UserStack},
    },
};
use core::{
    alloc::Layout,
    arch::naked_asm,
    cmp::min,
    fmt::Display,
    iter::Map,
    mem,
    ptr::{self, null, null_mut},
    sync::atomic::{AtomicU32, Ordering},
};

unsafe extern "C" {
    pub unsafe static _tdata_start: usize;
    pub unsafe static _tdata_end: usize;
    pub unsafe static _tbss_start: usize;
    pub unsafe static _tbss_end: usize;
}

#[repr(usize)]
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
impl From<usize> for Priority {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::RealTime,
            1 => Self::High,
            2 => Self::Normal,
            3 => Self::Background,
            _ => Self::Idle,
        }
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreadId {
    pub pid: u32,
    pub tid: u32,
}
impl ThreadId {
    /// This identifier is always reserved for the main thread of the kernel process
    pub const fn main() -> Self {
        Self { pid: 0, tid: 0 }
    }
}

#[derive(Debug)]
pub struct Thread {
    /// The thread identifier. TIDs are local to the owner process.
    pub id: ThreadId,
    /// information about the stack goes here. Contains both the stack pointer and thread pointer
    pub context: Context,
    /// Thread name for debugging
    name: [u8; 16],

    /// Execution stage of the thread
    pub state: State,
    /// Relative execution priority of the thread, 0 is the highest priority
    pub priority: Priority,

    /// How long the thread has been alive in cpu cycles
    lifetime: u64,

    /// Kernel Stack
    kernel_stack: KernelStack,
    /// User stack for threads part of user processes. This is mapped in virtual memory
    user_stack: Option<UserStack>,
}
/// Records the last used thread id. IDs are assigned incrementally, every id this value or greater is unused

impl Thread {
    /// Returns a preconfigured main thread. Does not have an entry point. Does not allocate stack or tls in memory.
    pub fn main() -> Thread {
        let kernel_stack = KernelStack::alloc().unwrap();
        Self {
            id: ThreadId::main(), // main thread
            name: Self::name_from_str("main"),
            state: State::Running, // The main thread creates itself, hence it is running
            context: Context::empty(),
            priority: Priority::Normal,
            lifetime: 0,
            kernel_stack,
            user_stack: None,
        }
    }
    /// attempts to create a new thread and allocate its stack in kernel and userspace
    pub fn new(
        pid: u32,
        tid: u32,
        name: &str,
        priority: Priority,
        entry: usize,
    ) -> Result<Self, ThreadError> {
        let id = ThreadId { pid, tid };
        let name = Self::name_from_str(name);
        let kernel_stack = KernelStack::alloc().ok_or(ThreadError::Other(
            "Failed to allocate Kernel Stack".to_owned(),
        ))?;
        let user_stack = UserStack::alloc(id);
        let context = Context::new(entry, kernel_stack.top(), kernel_stack.tls().addr());
        Ok(Self {
            id,
            context: Context::empty(),
            name,
            state: State::Waiting,
            priority,
            lifetime: 0,
            kernel_stack,
            user_stack,
        })
    }
    /// Creates and enques a thread. Returns the unique ID of the thread or None if queing failed.
    pub fn spawn(pid: u32, name: &str, priority: Priority, entry: usize) -> Option<ThreadId> {
        // let process = PROCESS_TABLE.wait().get();
        // let mut new = Self::new(pid, name, priority, entry).ok()?;
        // if let Err(error) = new.map_kernel_stack() {
        //     warn!("Failed to construct thread: {}", error);
        //     drop(new); // deallocate the memory on fail
        //     return None;
        // }
        // unsafe {
        //     let context = Context::new(entry, new.stack_bottom.addr(), new.tls_start.addr());
        //     new.context = context;
        //     new.state = State::Ready
        // }

        // let boxed = Box::new(new);
        // let box_ptr = Box::into_raw(boxed);

        // let tid = TID_ALLOCATOR.fetch_add(1, Ordering::Release);
        // match SCHEDULER.wait().lock().schedule(box_ptr) {
        //     Ok(_) => return Some(tid),
        //     Err(err) => {
        //         warn!("{err}: Failed to queue thread: '{name}'");
        //         let _ = unsafe { Box::from_raw(box_ptr) };
        //         return None;
        //     }
        // }
        None
    }
    /// Tries to spawn a new thread under the current running process. Returns [`None`] if the current process is *null*
    pub fn spawn_local(name: &str, entry: usize) -> Option<ThreadId> {
        let pptr = CURRENT_PROCESS.load(Ordering::Acquire);
        if pptr.is_null() {
            return None;
        }
        let pid = unsafe { (*pptr).pid() };
        Thread::spawn(pid, name, Priority::Normal, entry)
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
    pub fn tid(&self) -> u32 {
        self.id.tid
    }
    pub fn pid(&self) -> u32 {
        self.id.pid
    }
    /// Whether this thread is the main thread with id 0.
    pub fn is_main(&self) -> bool {
        self.id == ThreadId::main()
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        self.kernel_stack.free();
    }
}

// forcing cross thread accessibility
unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}

impl Display for Thread {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{}@{:x}",
            self.get_name_str(),
            self.kernel_stack.top(),
        ))
    }
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
    pub fn new(entry: usize, stack_bottom: usize, tls_start: usize) -> Self {
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
            sched.terminate_running();
            sched.run_next()
        };
        if let Ok((old_sp, new_sp, satp)) = next {
            switch_to(satp, old_sp, new_sp);
        }
        loop {
            core::arch::asm!("wfi");
        }
    }
}
