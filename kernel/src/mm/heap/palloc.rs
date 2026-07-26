use core::{
    ptr::{null, null_mut},
    sync::atomic::{AtomicUsize, Ordering, fence},
};

use crate::{console::writer::println, println};

const PAGE_SIZE: usize = 0x1000;

pub static FREE_PAGES: AtomicUsize = AtomicUsize::new(0);

static mut PAGE_HEAD: *mut PageHeader = null_mut();

pub fn init_page_allocator(kram_start: *const u8, kram_end: *const u8) {
    unsafe {
        let align = PAGE_SIZE - 1;
        let start_addr = (kram_start.addr() + align) & !align;
        let end_addr = (kram_end.addr() - PAGE_SIZE) & !align;

        let mut cur_ptr = end_addr as *const u8;
        let start_ptr = start_addr as *const u8;

        while start_ptr < cur_ptr {
            free(cur_ptr);
            cur_ptr = cur_ptr.sub(PAGE_SIZE);
        }
    }
}
#[repr(C, align(0x1000))]
#[derive(Clone, Copy, Default, Debug)]
struct PageHeader {
    next: *mut PageHeader,
}

/// allocates a 4KiB page
pub fn free(phys_addr: *const u8) {
    unsafe {
        let mut page = phys_addr as *mut PageHeader;
        (*page).next = PAGE_HEAD;
        PAGE_HEAD = page;
        FREE_PAGES.fetch_add(1, Ordering::Relaxed);
    }
}
pub fn alloc() -> *mut u8 {
    unsafe {
        let page = PAGE_HEAD;
        if !page.is_null() {
            PAGE_HEAD = (*page).next;
            FREE_PAGES.fetch_sub(1, Ordering::Relaxed);
            return page as *mut u8;
        }
        null_mut()
    }
}
