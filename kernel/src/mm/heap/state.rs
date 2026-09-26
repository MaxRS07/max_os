use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use log::error;
use mem::{allocator::HeapAllocator, error::MemoryError};

use crate::mm::heap::{BOOT_ALLOCATOR, KERNEL_ALLOCATOR};

static BOOT_HEAP_SIZE: usize = 0x20000; // 128 KiB

#[derive(Clone, Copy, Debug)]
pub enum AllocatorState {
    Uninitialized,
    Boot,
    LinkedList,
}

unsafe impl GlobalAlloc for AllocatorState {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self {
            Self::Uninitialized => Err(MemoryError::NotInitialized(
                "alloc failed: GlobalAlloc is not yet initialized",
            )),
            Self::Boot => BOOT_ALLOCATOR.wait().alloc(layout),
            Self::LinkedList => KERNEL_ALLOCATOR.wait().alloc(layout),
        }
        .unwrap_or_else(|err| {
            error!("{err}");
            null_mut()
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match self {
            Self::Uninitialized => Err(MemoryError::NotInitialized(
                "alloc failed: GlobalAlloc is not yet initialized",
            )),
            Self::Boot => BOOT_ALLOCATOR.wait().free(ptr, layout),
            Self::LinkedList => KERNEL_ALLOCATOR.wait().free(ptr, layout),
        }
        .inspect_err(|err| {
            error!("{err}");
        });
    }
}
