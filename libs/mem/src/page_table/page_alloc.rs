use core::{
    ptr::{null, null_mut},
    sync::atomic::{AtomicUsize, Ordering, fence},
};

use sync::mutex::{self, Mutex};

use crate::error::MemoryError;

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
    fn alloc(&self) -> Result<*mut u8, MemoryError>;
    fn free(&self, ptr: *mut u8) -> Result<(), MemoryError>;
}

#[repr(C, align(0x1000))]
#[derive(Clone, Copy, Default, Debug)]
struct PageHeader {
    next: *mut PageHeader,
}

pub struct PageAllocator {
    /// Pointer to the page head
    page_head: *mut PageHeader,
    free_pages: AtomicUsize,
}
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
    /// allocates a 4KiB page
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
    /// frees the page at physical address
    pub fn free(&mut self, page: *mut u8) -> Result<(), MemoryError> {
        if page.is_null() {
            return Err(MemoryError::InvalidAddress("Attempt to free null page"));
        }
        unsafe {
            let page = page as *mut PageHeader;
            (*page).next = self.page_head;
            self.page_head = page;
            self.free_pages.fetch_add(1, Ordering::Relaxed);
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
        self.lock().free(ptr)
    }
}
