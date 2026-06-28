use core::alloc::GlobalAlloc;

pub static mut BLOCK_HEAD: *mut BlockHeader = core::ptr::null_mut();

#[derive(Clone, Copy, Debug, Default)]
pub struct BlockHeader {
    pub size: usize,
    pub free: bool,
    pub next: Option<*mut BlockHeader>,
    /// for quicker merging on free
    pub prev: Option<*mut BlockHeader>,
}
impl BlockHeader {
    /// Returns `size_of::<BlockHeader>()`
    pub fn size() -> usize {
        core::mem::size_of::<BlockHeader>()
    }
}

pub struct LinkedListAllocator;

unsafe impl GlobalAlloc for LinkedListAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        LinkedListAllocator::alloc(layout.size()).unwrap()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        unsafe {
            LinkedListAllocator::free(ptr);
        }
    }
}

impl LinkedListAllocator {
    pub fn alloc(size: usize) -> Option<*mut u8> {
        let aligned_size = (size + 7) & !7; // always align to 8

        let mut maybe_current = Some(unsafe { BLOCK_HEAD });

        while let Some(current_ptr) = maybe_current {
            let current_block = unsafe { &mut *current_ptr };
            // check if block can fit header and can fit payload
            if current_block.free && current_block.size >= size {
                // dont waste bytes
                if current_block.size > aligned_size + BlockHeader::size() + 8 {
                    unsafe {
                        let new_block = current_ptr.add(1).byte_add(aligned_size);
                        (*new_block).free = true;
                        (*new_block).size = current_block.size - aligned_size - BlockHeader::size();
                        (*new_block).next = current_block.next;
                        (*new_block).prev = Some(current_ptr);

                        current_block.size = aligned_size;
                        current_block.next = Some(new_block);

                        return Some(current_ptr.add(1) as *mut u8); // skip sizeof header (payload ptr)
                    }
                }
                current_block.free = false;
            }
            maybe_current = current_block.next;
        }
        None
    }

    /// Frees your memory
    /// # Saftey
    /// Please only pass the start address of a payload here or kernel will crash
    pub unsafe fn free(block_ptr: *mut u8) {
        let header_start_ptr = unsafe { block_ptr.sub(BlockHeader::size()) };
        let header_ptr = header_start_ptr as *mut BlockHeader;
        let header_ref = unsafe { &mut *header_ptr };
        header_ref.free = true;
        if let Some(next_ptr) = header_ref.next
            && unsafe { (*next_ptr).free }
        {
            let next_block = unsafe { &mut *next_ptr };
            header_ref.size += next_block.size + BlockHeader::size();
            header_ref.next = next_block.next;

            // next block needs to point here
            if let Some(next_next_ptr) = header_ref.next {
                unsafe {
                    (*next_next_ptr).prev = Some(header_ptr);
                }
            }
        }
        if let Some(prev_ptr) = header_ref.prev
            && unsafe { (*prev_ptr).free }
        {
            let prev_block = unsafe { &mut *prev_ptr };
            prev_block.size += BlockHeader::size() + header_ref.size;
            prev_block.next = header_ref.next;

            if let Some(next_ptr) = header_ref.next {
                unsafe {
                    (*next_ptr).prev = Some(prev_ptr);
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
        block_start.next = None;
        block_start.prev = None;
    }
}
