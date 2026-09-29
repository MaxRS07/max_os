use core::{
    cell::{OnceCell, UnsafeCell},
    fmt::Debug,
    ptr::{addr_of, null, null_mut},
};

use alloc::alloc::{GlobalAlloc, Layout, alloc};
use log::{debug, error, info, warn};
use mem::{
    allocator::{bump::BumpAllocator, linked_list::LinkedListAllocator},
    error::MemoryError,
    page_table::page_alloc::PageAllocator,
};
use sync::{mutex::Mutex, oncelock::OnceLock};

use crate::mm::{
    BOOT_HEAP_SIZE,
    heap::{kalloc::ALLOCATOR, state::AllocatorState},
};

pub mod kalloc;
pub mod state;

/// Pre-boot kernel allocator
pub static BOOT_ALLOCATOR: OnceLock<Mutex<BumpAllocator>> = OnceLock::new();

pub static PAGE_ALLOCATOR: OnceLock<Mutex<PageAllocator>> = OnceLock::new();
pub static KERNEL_ALLOCATOR: OnceLock<Mutex<LinkedListAllocator>> = OnceLock::new();

// Result wrappers for getting mutable ref of global allocators
unsafe extern "C" {
    // end of linker memory
    unsafe static _end: usize;
}

pub fn setup_boot_mem() {
    // I HAve no idea how big this is supposed to be yet
    BOOT_ALLOCATOR.set(Mutex::new(BumpAllocator::new(
        &raw const _end as usize,
        BOOT_HEAP_SIZE,
    )));
    ALLOCATOR.change_state(AllocatorState::Boot);
}

// RAM base for the QEMU virt board (see linker.ld: `. = 0x80000000`)
const RAM_BASE: usize = 0x8000_0000;

pub fn setup_system_mem(total_size: usize) {
    unsafe {
        let kmem_start = addr_of!(_end).add(BOOT_HEAP_SIZE) as *const u8;
        let kmem_end = (RAM_BASE + total_size) as *const u8;
        // `total_size` is the whole RAM region
        PAGE_ALLOCATOR.set(Mutex::new(PageAllocator::new(kmem_start, kmem_end)));
        debug!("Intialized page allocator");

        match LinkedListAllocator::new(PAGE_ALLOCATOR.wait(), kmem_start) {
            Ok(lla) => {
                KERNEL_ALLOCATOR.set(Mutex::new(lla));
                debug!("Intialized kernel allocator")
            }
            // if paging fails just crash
            Err(err) => panic!("{err}"),
        }
    }
    // hand the global allocator over to the linked-list heap now that RAM is mapped
    ALLOCATOR.change_state(AllocatorState::LinkedList);
}
