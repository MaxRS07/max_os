use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::{null, null_mut},
};

use log::debug;

use crate::mm::heap::palloc;

// TODO: stop using public mutable statics
pub static mut BLOCK_HEAD: *mut BlockHeader = core::ptr::null_mut();
pub static mut BLOCK_TAIL: *mut BlockHeader = core::ptr::null_mut();

static HEADER_SIZE: usize = size_of::<BlockHeader>();

#[derive(Clone, Copy, Debug, Default)]
pub struct BlockHeader {
    pub size: usize,
    pub free: bool,
    pub next: *mut BlockHeader,
    /// for quicker merging on free
    pub prev: *mut BlockHeader,
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
        let size = layout.size().max(1);
        let align = layout.align().max(HEADER_SIZE);

        let mut cur = unsafe { BLOCK_HEAD };
        while !cur.is_null() {
            let block = unsafe { &mut *cur };
            // check if block can fit header and can fit payload
            if !block.free {
                cur = block.next;
                continue;
            }
            // start address of current block
            let base = cur.addr();

            // end address of current block
            let block_end = base + block.size + HEADER_SIZE;

            // next aligned address
            let mut payload = (base + HEADER_SIZE + align - 1) & !(align - 1);

            // make sure payload doesnt collide with the current header, if it does, push forward by align (at least HEADER_SIZE) preventing collision
            if payload - HEADER_SIZE != base && (payload - HEADER_SIZE) - base < HEADER_SIZE {
                payload += align;
            }

            let alloc_hdr = payload - HEADER_SIZE;

            if payload + size <= block_end {
                unsafe {
                    if alloc_hdr != base {
                        block.size = alloc_hdr - (base + HEADER_SIZE);
                        let ah = alloc_hdr as *mut BlockHeader;
                        (*ah).prev = cur;
                        (*ah).next = block.next;
                        if !block.next.is_null() {
                            (*block.next).prev = ah;
                        }
                        block.next = ah;
                    }

                    let ab = &mut *(alloc_hdr as *mut BlockHeader);

                    let end = payload + size;
                    if block_end - end > HEADER_SIZE {
                        let th = end as *mut BlockHeader;
                        (*th).free = true;
                        (*th).size = block_end - end - HEADER_SIZE;
                        (*th).next = ab.next;
                        (*th).prev = alloc_hdr as *mut BlockHeader;

                        if BLOCK_TAIL == cur || BLOCK_TAIL == alloc_hdr as *mut BlockHeader {
                            BLOCK_TAIL = th
                        }

                        if !ab.next.is_null() {
                            (*ab.next).prev = th;
                        }
                        ab.next = th;
                        ab.size = size;
                    } else {
                        ab.size = block_end - payload;
                    }

                    ab.free = false;
                    return payload as *mut u8;
                }
            }
            cur = block.next;
        }
        // Out of space. Allocate a new page.

        let new_page = palloc::alloc();
        if !new_page.is_null() {
            let new_block = new_page as *mut BlockHeader;
            unsafe {
                (*new_block).size = 0x1000 - HEADER_SIZE;
                (*new_block).free = true;
                (*new_block).prev = null_mut();
                (*new_block).next = null_mut();

                if !BLOCK_TAIL.is_null() {
                    (*BLOCK_TAIL).next = new_block;
                    (*new_block).prev = BLOCK_TAIL;
                    BLOCK_TAIL = new_block;
                } else {
                    BLOCK_HEAD = new_block;
                    BLOCK_TAIL = new_block;
                }
            }
            return Self::alloc(layout);
        }
        null_mut()
    }

    /// Frees your memory
    /// # Safety
    /// Only pass the start address of a payload
    pub fn free(block_ptr: *mut u8) {
        if block_ptr.is_null() {
            return;
        }

        unsafe {
            let header_start_ptr = block_ptr.sub(HEADER_SIZE);
            let header_ptr = header_start_ptr as *mut BlockHeader;
            let header_ref = &mut *header_ptr;
            header_ref.free = true;

            // merge right
            if !header_ref.next.is_null() && (*header_ref.next).free {
                let next_block_ptr = header_ref.next;

                // check if they ate physically contiguous in memory
                let current_block_end = (header_ptr as usize) + HEADER_SIZE + header_ref.size;
                if current_block_end == (next_block_ptr as usize) {
                    // if the right block was the tail, this is now the tail
                    if BLOCK_TAIL == next_block_ptr {
                        BLOCK_TAIL = header_ptr;
                    }

                    header_ref.size += (*next_block_ptr).size + HEADER_SIZE;
                    header_ref.next = (*next_block_ptr).next;

                    if !header_ref.next.is_null() {
                        (*header_ref.next).prev = header_ptr;
                    }
                }
            }

            // left merge
            if !header_ref.prev.is_null() && (*header_ref.prev).free {
                let prev_block_ptr = header_ref.prev;

                // again check if they are contiguous
                let prev_block_end =
                    (prev_block_ptr as usize) + HEADER_SIZE + (*prev_block_ptr).size;
                if prev_block_end == (header_ptr as usize) {
                    // if this is the tail the entire block is the tail now
                    if BLOCK_TAIL == header_ptr {
                        BLOCK_TAIL = prev_block_ptr;
                    }

                    (*prev_block_ptr).size += HEADER_SIZE + header_ref.size;
                    (*prev_block_ptr).next = header_ref.next;

                    if !header_ref.next.is_null() {
                        (*header_ref.next).prev = prev_block_ptr;
                    }
                }
            }
        }
    }
}

/// Initalizes the linked list allocator over the first 4KiB in RAM
///
/// # Safety
///
/// .
pub unsafe fn init(mm_start: *const u8) {
    let block_start_ptr = palloc::alloc() as *mut BlockHeader;
    unsafe {
        BLOCK_HEAD = block_start_ptr;
        BLOCK_TAIL = block_start_ptr;

        let block_start = &mut *block_start_ptr;
        block_start.free = true;
        block_start.size = 0x1000;
        block_start.next = null_mut();
        block_start.prev = null_mut();
    }
}
