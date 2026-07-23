use core::ptr::{null, null_mut};

const PAGE_SIZE: usize = 0x1000;

static mut PAGE_HEAD: *mut PageHeader;

pub fn init_page_allocator(ram_start: *const u8, ram_end: *const u8) {
    unsafe {
        let align = PAGE_SIZE - 1;
        let start_addr = (ram_start.addr() + align) & !align;
        let end_addr = ram_end.addr() & !align;

        let mut cur_ptr = start_addr as *const u8;
        let end_ptr = end_addr as *const u8;

        while cur_ptr < end_ptr {
            free(cur_ptr);
            cur_ptr.add(PAGE_SIZE);
        }
    }
}
#[repr(C, align(0x1000))]
#[derive(Clone, Copy, Default, Debug)]
struct PageHeader {
    next: *mut PageHeader,
}

/// allocates a 4KiB page
pub fn free(phys_addr: *const u8) -> *mut u8 {
    unsafe {
        let mut page = phys_addr as *mut PageHeader;

        (*page).next = PAGE_HEAD;
        PAGE_HEAD = page
    }
    null_mut()
}
pub fn alloc() -> *mut u8 {
    unsafe {
        let page = PAGE_HEAD;
        if !page.is_null() {
            PAGE_HEAD = (*page).next;
            return page as *mut u8;
        }
        null_mut()
    }
}
