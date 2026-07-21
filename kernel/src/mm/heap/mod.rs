use core::{cell::UnsafeCell, fmt::Debug, ptr::null_mut};

use alloc::alloc::{GlobalAlloc, Layout};
use log::{info, warn};

use crate::mm::heap::{boot::BootAllocator, linked_list::LinkedListAllocator};

pub mod boot;
pub mod linked_list;
pub mod page_table;

unsafe extern "C" {
    // end of linker memory
    unsafe static _end: usize;
}

static BOOT_HEAP_SIZE: usize = 0x20000; // 128 KiB

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

#[derive(Clone, Copy, Debug)]
pub enum AllocatorState {
    Uninitialized,
    Boot,
    LinkedList,
}

unsafe impl GlobalAlloc for AllocatorState {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self {
            Self::Uninitialized => {
                warn!("alloc failed: GlobalAlloc is not yet initialized");
                null_mut()
            }
            Self::Boot => unsafe { BootAllocator.alloc(layout) },
            Self::LinkedList => unsafe { LinkedListAllocator.alloc(layout) },
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match self {
            Self::Uninitialized => {
                warn!("dealloc failed: GlobalAlloc is not yet initialized");
            }
            Self::Boot => unsafe { BootAllocator.dealloc(ptr, layout) },
            Self::LinkedList => unsafe { LinkedListAllocator.dealloc(ptr, layout) },
        }
    }
}

pub fn setup_boot_mem() {
    boot::init_boot_allocator();
    ALLOCATOR.change_state(AllocatorState::Boot);
}

// RAM base for the QEMU virt board (see linker.ld: `. = 0x80000000`)
const RAM_BASE: usize = 0x8000_0000;

pub fn setup_system_mem(total_size: usize) {
    unsafe {
        let link_start = core::ptr::addr_of!(_end) as usize + BOOT_HEAP_SIZE;
        // `total_size` is the whole RAM region
        let available = (RAM_BASE + total_size).saturating_sub(link_start);
        linked_list::init(link_start as *const u8, available);
    }
    // hand the global allocator over to the linked-list heap now that RAM is mapped
    ALLOCATOR.change_state(AllocatorState::LinkedList);
}
