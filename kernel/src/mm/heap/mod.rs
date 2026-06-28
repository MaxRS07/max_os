use alloc::alloc::{GlobalAlloc, Layout};

use crate::mm::heap::linked_list::LinkedListAllocator;

pub mod bump;
pub mod linked_list;

unsafe extern "C" {
    // end of linker memory
    static _end: usize;
}
// 4KiB max size. LL takes ram after this
static BOOT_HEAP_SIZE: usize = 0x1000;

pub struct DummyAllocator;

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        panic!("Allocator not yet implemented")
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("Allocator not yet implemented")
    }
}

#[global_allocator]
pub static ALLOCATOR: LinkedListAllocator = LinkedListAllocator;

pub fn init(total_size: usize) {
    bump::init();

    unsafe {
        let link_start = _end + BOOT_HEAP_SIZE;
        linked_list::init(link_start as *const u8, total_size);
    }
}
