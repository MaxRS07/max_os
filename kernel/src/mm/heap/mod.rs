use core::{
    cell::{OnceCell, UnsafeCell},
    fmt::Debug,
    ptr::{addr_of, null_mut},
};

use alloc::alloc::{GlobalAlloc, Layout};
use log::{debug, info, warn};

use crate::mm::heap::{boot::BootAllocator, kalloc::LinkedListAllocator, palloc::PageAllocator};

pub mod boot;
pub mod kalloc;
pub mod palloc;

pub static mut PAGE_ALLOCATOR: OnceCell<PageAllocator> = OnceCell::new();

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
        let kmem_start = addr_of!(_end).add(BOOT_HEAP_SIZE) as *const u8;
        let kmem_end = (RAM_BASE + total_size) as *const u8;
        // `total_size` is the whole RAM region
        PAGE_ALLOCATOR.set(PageAllocator::new(kmem_start, kmem_end));
        debug!("Intialized page allocator");

        match LinkedListAllocator::new(kmem_start) {
            Some(_) => debug!("Intialized kernel allocator"),
            // if paging fails just crash
            None => panic!("Paging failed, giving up"),
        }
    }
    // hand the global allocator over to the linked-list heap now that RAM is mapped
    ALLOCATOR.change_state(AllocatorState::LinkedList);
}
