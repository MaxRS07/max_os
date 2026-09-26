use core::{
    ptr::null_mut,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::error::MemoryError;

const PAGE_SIZE: usize = 0x1000;

#[repr(C, align(0x1000))]
#[derive(Clone, Copy, Default, Debug)]
struct PageHeader {
    next: *mut PageHeader,
}

/// Freelist-based physical page allocator. Hands out and reclaims 4KiB pages.
///
/// Callers that need paged memory (e.g. [`super::linked_list::LinkedListAllocator`],
/// [`crate::page_table::address_space::AddressSpace`]) take a `&'static
/// sync::spinmutex::Mutex<PageAllocator>` rather than reaching for a global, so this
/// crate stays free of statics of its own; the owning binary decides where the
/// singleton lives.
pub struct PageAllocator {
    page_head: *mut PageHeader,
    free_pages: AtomicUsize,
}

unsafe impl Send for PageAllocator {}

impl PageAllocator {
    pub fn new(kram_start: *const u8, kram_end: *const u8) -> Self {
        let mut new = Self {
            page_head: null_mut(),
            free_pages: AtomicUsize::new(0),
        };
        unsafe {
            let align = PAGE_SIZE - 1;
            let start_addr = (kram_start.addr() + align) & !align;
            let end_addr = (kram_end.addr() - PAGE_SIZE) & !align;

            let mut cur_ptr = end_addr as *const u8;
            let start_ptr = start_addr as *const u8;

            while start_ptr < cur_ptr {
                new.free(cur_ptr);
                cur_ptr = cur_ptr.sub(PAGE_SIZE);
            }
        }
        new
    }

    /// Returns a 4KiB page to the freelist
    pub fn free(&mut self, phys_addr: *const u8) {
        unsafe {
            let page = phys_addr as *mut PageHeader;
            (*page).next = self.page_head;
            self.page_head = page;
            self.free_pages.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Allocates a 4KiB page
    pub fn alloc(&mut self) -> Result<*mut u8, MemoryError> {
        unsafe {
            let page = self.page_head;
            if !page.is_null() {
                self.page_head = (*page).next;
                self.free_pages.fetch_sub(1, Ordering::Relaxed);
                return Ok(page as *mut u8);
            }
        }
        Err(MemoryError::OutOfMemory("No more pages"))
    }

    pub fn free_pages(&self) -> usize {
        self.free_pages.load(Ordering::Acquire)
    }
}
