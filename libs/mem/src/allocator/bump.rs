use core::alloc::Layout;

use sync::mutex::Mutex;

use crate::{align::aligned_up, allocator::HeapAllocator, error::MemoryError};

/// Preboot allocator with no overhead. Makes allocations by shifting a physical allocation pointer.
///
/// **Note:** Bump allocations are untracked meaning they cannot be freed.
pub struct BumpAllocator {
    start: usize,
    current: usize,
    size: usize,
}

impl BumpAllocator {
    pub fn new(start: usize, size: usize) -> Self {
        return Self {
            start,
            current: 0,
            size,
        };
    }
    /// Allocates a memory block with the specified layout and increments [`Self::current`] to the next specified alignment
    pub fn bump(&mut self, layout: Layout) -> Result<*mut u8, MemoryError> {
        let alloc_size = layout.size();
        let align = layout.align();

        let bump_pos = aligned_up(self.current, align);

        if bump_pos + alloc_size > self.start + self.size {
            return Err(MemoryError::OutOfMemory(
                "Allocation of specified size/layout cannot fit in bump allocator",
            ));
        }
        self.current = bump_pos + alloc_size;

        Ok(bump_pos as *mut u8)
    }
}

impl HeapAllocator for Mutex<BumpAllocator> {
    fn alloc(&self, layout: Layout) -> Result<*mut u8, MemoryError> {
        self.lock().bump(layout)
    }
    /// This free does nothing
    fn free(&self, _ptr: *mut u8, _layout: Layout) -> Result<(), MemoryError> {
        Err(MemoryError::AccessViolation(
            "Attempted to free memory using the bump allocator",
        ))
    }
}
