use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{null, null_mut};
use core::sync::atomic::{AtomicUsize, Ordering};

use log::{info, warn};

/// Simple bump allocator for boot instructions, before ram
use crate::mm::error::MemoryError;
use crate::mm::heap::{_end, BOOT_HEAP_SIZE};
use crate::println;

static CURRENT_ADDR: AtomicUsize = AtomicUsize::new(0);

pub fn init_boot_allocator() {
    // `_end` is a linker symbol: we want its address, not the value stored there.
    let end = core::ptr::addr_of!(_end) as usize;
    CURRENT_ADDR.store(end, Ordering::Release);
}
pub struct BootAllocator;

impl BootAllocator {
    /// allocates `size * size_of::<T>()` bytes in the bump.
    /// # Safety
    pub unsafe fn bump_alloc(layout: Layout) -> Result<*mut u8, MemoryError> {
        let current = CURRENT_ADDR.load(Ordering::Acquire);
        // round `current` up to the requested alignment (align is a power of two)
        let aligned = (current + layout.align() - 1) & !(layout.align() - 1);
        let new_current = aligned + layout.size();
        let max_heap = (core::ptr::addr_of!(_end) as usize) + BOOT_HEAP_SIZE;
        if new_current > max_heap {
            return Err(MemoryError::OutOfMemory("bump is limited to 4KiB"));
        }
        CURRENT_ADDR.store(new_current, Ordering::Release);
        Ok(aligned as *mut u8)
    }
}
unsafe impl GlobalAlloc for BootAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { Self::bump_alloc(layout).unwrap_or(null_mut()) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // warn!("Cannot deallocate boot memory")
    }
}
