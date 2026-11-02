use alloc::{borrow::ToOwned, format};
use mem::{
    error::MemoryError,
    page_table::{
        address_space::{AddressSpace, Addresser},
        permissions::PagePermissions,
    },
};

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
    pub fn alloc(tid: usize, space: &mut AddressSpace) -> Result<Self, MemoryError> {
        let region_size = USER_GUARD_SIZE + USER_STACK_SIZE;
        let top = PROCESS_STACK_TOP - tid * region_size;
        let base = PROCESS_STACK_TOP - USER_STACK_SIZE;
        // guard is unmapped, prevents thread stack from writing to another. TODO: handle allocations with >0x1000 layouts
        let guard = top - region_size;
        space.map_pages(base, STACK_PAGES, 0x0, PagePermissions::RW)?;
        Ok(Self { base, top, guard })
    }
    pub fn free(&mut self, space: &mut AddressSpace) -> Result<(), MemoryError> {
        space.unmap_pages(self.base, STACK_PAGES)
    }
}
