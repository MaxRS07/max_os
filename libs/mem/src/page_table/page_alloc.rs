use core::{
    ptr::{null, null_mut},
    sync::atomic::{AtomicUsize, Ordering, fence},
};

use sync::mutex::{self, Mutex};

use crate::{
    align::{aligned_down, aligned_up},
    error::MemoryError,
};

const PAGE_SIZE: usize = 0x1000;

/// Page allocator trait, responsible for freeing and allocating 4kib pages. Expects types with thread safe interior mutability only.
///
/// **NOTE** Propagates [`MemoryError`]s at the libs level.
/// ## Example
/// ```
/// impl Pager for sync::Mutex<T> {
///     ...
/// }
/// ```
pub trait Pager {
    /// allocates a free page and returns the pointer to the start of the page
    fn alloc(&self) -> Result<*mut u8, MemoryError>;
    /// Frees a page by the page pointer. Should be aligned to 0x1000.
    fn free(&self, ptr: *mut u8) -> Result<(), MemoryError>;
    /// Gets the total number of free pages on the page allocator. Does not require lock, do not check for atomic operations unless lock is acuired on this.
    fn page_count(&self) -> usize;
}

#[repr(C, align(0x1000))]
#[derive(Clone, Copy, Default, Debug)]
struct PageHeader {
    next: *mut PageHeader,
}

pub struct PageAllocator {
    /// Pointer to the page head
    page_head: *mut PageHeader,
    free_pages: usize,
}
impl PageAllocator {
    pub fn new(kram_start: *const u8, kram_end: *const u8) -> Self {
        let mut new = Self {
            page_head: null_mut(),
            free_pages: 0,
        };
        unsafe {
            let align = PAGE_SIZE - 1;
            let start_addr = aligned_up(kram_start.addr(), align);
            let end_addr = aligned_down(kram_end.addr() - PAGE_SIZE, align);

            let mut cur_ptr = end_addr as *mut u8;
            let start_ptr = start_addr as *mut u8;

            while start_ptr < cur_ptr {
                // clears all tables
                new.free(cur_ptr);
                cur_ptr = cur_ptr.sub(PAGE_SIZE);
            }
        }
        new
    }
    /// allocates a 4KiB page
    pub fn alloc(&mut self) -> Result<*mut u8, MemoryError> {
        if self.free_pages == 0 {
            return Err(MemoryError::OutOfMemory("No more pages"));
        }
        unsafe {
            let page = self.page_head;
            if !page.is_null() {
                self.page_head = (*page).next;
                self.free_pages -= 1;
                return Ok(page as *mut u8);
            }
            Err(MemoryError::InvalidAddress(
                "Attempted to allocate null page",
            ))
        }
    }
    /// frees the page at physical address
    pub fn free(&mut self, page: *mut u8) -> Result<(), MemoryError> {
        if page.is_null() {
            return Err(MemoryError::InvalidAddress("Attempt to free null page"));
        }
        unsafe {
            let page = page as *mut PageHeader;
            (*page).next = self.page_head;
            self.page_head = page;
            self.free_pages += 1;
        }
        Ok(())
    }
}

impl Pager for Mutex<PageAllocator> {
    fn alloc(&self) -> Result<*mut u8, MemoryError> {
        // TODO: MAke sure this doesnt spin forever
        self.lock().alloc()
    }
    fn free(&self, ptr: *mut u8) -> Result<(), MemoryError> {
        // TODO: this also should break at some point
        let ptr = aligned_down(ptr as usize, 0x1000) as *mut u8;
        self.lock().free(ptr)
    }
    fn page_count(&self) -> usize {
        self.as_ref().free_pages
    }
}
