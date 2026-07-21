use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use log::debug;

// TODO: stop using public mutable statics
pub static mut BLOCK_HEAD: *mut BlockHeader = core::ptr::null_mut();

#[derive(Clone, Copy, Debug, Default)]
pub struct BlockHeader {
    pub size: usize,
    pub free: bool,
    pub next: *mut BlockHeader,
    /// for quicker merging on free
    pub prev: *mut BlockHeader,
}
impl BlockHeader {
    /// Returns `size_of::<BlockHeader>()`
    pub fn size() -> usize {
        core::mem::size_of::<BlockHeader>()
    }
}

pub struct LinkedListAllocator;

unsafe impl GlobalAlloc for LinkedListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LinkedListAllocator::alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            LinkedListAllocator::free(ptr);
        }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.alloc(layout) };
        if !ptr.is_null() {
            unsafe { core::ptr::write_bytes(ptr, 0, layout.size()) };
        }
        ptr
    }
}

impl LinkedListAllocator {
    pub fn alloc(layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        let aligned_size = (size + (align - 1)) & !(align - 1); // always align to 8

        let mut maybe_current = unsafe { BLOCK_HEAD };

        while !maybe_current.is_null() {
            let current_block = unsafe { &mut *maybe_current };
            // check if block can fit header and can fit payload
            if current_block.free && current_block.size >= size {
                if current_block.size > aligned_size + BlockHeader::size() + 8 {
                    unsafe {
                        let new_block = maybe_current.byte_add(BlockHeader::size() + aligned_size);
                        (*new_block).free = true;
                        (*new_block).size = current_block.size - aligned_size - BlockHeader::size();
                        (*new_block).next = current_block.next;
                        (*new_block).prev = maybe_current;

                        if !current_block.next.is_null() {
                            (*current_block.next).prev = new_block;
                        }

                        current_block.size = aligned_size;
                        current_block.next = new_block;
                    }
                }
                current_block.free = false; // was missing in the no-split path
                return unsafe { maybe_current.add(1) as *mut u8 }; // was missing in the no-split path
            }
            maybe_current = current_block.next;
        }
        null_mut()
    }

    /// Frees your memory
    /// # Safety
    /// Please only pass the start address of a payload here or kernel will crash
    pub unsafe fn free(block_ptr: *mut u8) {
        let header_start_ptr = unsafe { block_ptr.sub(BlockHeader::size()) };
        let header_ptr = header_start_ptr as *mut BlockHeader;
        let header_ref = unsafe { &mut *header_ptr };
        header_ref.free = true;
        if !header_ref.next.is_null() && unsafe { (*header_ref.next).free } {
            let next_block = unsafe { &mut *header_ref.next };
            header_ref.size += next_block.size + BlockHeader::size();
            header_ref.next = next_block.next;

            // next block needs to point here
            if !header_ref.next.is_null() {
                unsafe {
                    (*header_ref.next).prev = header_ptr;
                }
            }
        }
        if !header_ref.prev.is_null() && unsafe { (*header_ref.prev).free } {
            let prev_block = unsafe { &mut *header_ref.prev };
            prev_block.size += BlockHeader::size() + header_ref.size;
            prev_block.next = header_ref.next;

            if !header_ref.next.is_null() {
                unsafe {
                    (*header_ref.next).prev = header_ref.prev;
                }
            }
        }
    }
}

/// .
///
/// # Safety
///
/// .
pub unsafe fn init(mm_start: *const u8, size: usize) {
    let block_start_ptr = mm_start as *mut BlockHeader;
    unsafe {
        BLOCK_HEAD = block_start_ptr;
        let block_start = &mut *block_start_ptr;
        block_start.free = true;
        block_start.size = size - BlockHeader::size();
        block_start.next = null_mut();
        block_start.prev = null_mut();
    }
}
