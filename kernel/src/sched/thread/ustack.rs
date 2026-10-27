use alloc::{borrow::ToOwned, format};

use crate::sched::{
    PROCESS_TABLE,
    error::ThreadError,
    process::ProcessTable,
    thread::thread::{Thread, ThreadId},
};

pub const PROCESS_STACK_TOP: usize = 0x8000_0000;
/// The user stack should be 12kib + 4kib guard page
pub const USER_STACK_SIZE: usize = 0x3000;
pub const USER_GUARD_SIZE: usize = 0x1000;
pub const STACK_PAGES: usize = USER_STACK_SIZE / 0x1000; // 3 + 1

/// The user stack allocation
#[derive(Debug)]
pub struct UserStack {
    /// The virtual address of the bottom of the stack
    base: usize,
    /// The virtual address of the top of the stack
    top: usize,
    guard: usize,
}
impl UserStack {
    pub fn alloc(id: ThreadId) -> Option<Self> {
        if id.pid == 0 {
            return None;
        }
        let pid = id.pid;
        let process_ptr =
            PROCESS_TABLE
                .wait()
                .lock()
                .get(&thread.pid())
                .ok_or(ThreadError::Other(format!(
                    "Invalid pid: {pid}, process not found"
                )))?;
        let process = unsafe { &mut *process };

        let offset = id.tid as usize;
        let region_size = USER_GUARD_SIZE + USER_STACK_SIZE;
        let top = PROCESS_STACK_TOP - (offset * region_size);
        let base = PROCESS_STACK_TOP - USER_STACK_SIZE;
        let guard = top - region_size;
        process
            .map_size(guard, STACK_PAGES, 0, 0, PagePermissions::RWXU) // map an extra page for the guard below the stack
            .map_err(|err| ThreadError::Other(err.to_string()))?;
        Some(Self { base, top, guard })
    }
}
