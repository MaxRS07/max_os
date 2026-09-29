use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    fmt::Debug,
};

use log::info;

use crate::mm::heap::AllocatorState;

#[global_allocator]
pub static ALLOCATOR: StatefulAllocator = StatefulAllocator::new();

pub struct StatefulAllocator {
    s: UnsafeCell<AllocatorState>,
}
impl StatefulAllocator {
    pub const fn new() -> Self {
        Self {
            s: UnsafeCell::new(AllocatorState::Uninitialized),
        }
    }
    pub fn change_state(&self, new: AllocatorState) {
        unsafe {
            info!("Changed allocator state to {:?}", new);
            let allocator_state = ALLOCATOR.s.get();
            (*allocator_state) = new;
        }
    }
}
impl Debug for StatefulAllocator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let state = unsafe { *self.s.get() };
        f.debug_struct("StatefulAllocator")
            .field("s", &state)
            .finish()
    }
}

impl Default for StatefulAllocator {
    fn default() -> Self {
        Self::new()
    }
}
unsafe impl Sync for StatefulAllocator {}
unsafe impl Send for StatefulAllocator {}

unsafe impl GlobalAlloc for StatefulAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { (*self.s.get()).alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { (*self.s.get()).dealloc(ptr, layout) }
    }
}
