use core::alloc::Layout;

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
    pub fn bump_alloc(&mut self, layout: Layout) -> Result<*mut u8, MemoryError> {
        let alloc_size = layout.size();
        let align = layout.align();

        let bump_pos = aligned_up(self.current, align);

        if bump_pos + alloc_size > self.size {
            return Err(MemoryError::OutOfMemory(
                "Allocation of size cannot fit in bump allocator",
            ));
        }
        self.current = bump_pos + alloc_size;

        Ok(bump_pos as *mut u8)
    }
}

impl HeapAllocator for BumpAllocator {
    fn alloc(&mut self, layout: Layout) -> Result<*mut u8, MemoryError> {
        self.bump_alloc(layout)
    }
    /// This free does nothing
    fn free(&mut self, _ptr: *mut u8, _layout: Layout) -> Result<(), MemoryError> {
        Err(MemoryError::AccessViolation(
            "Attempted to free memory using the bump allocator",
        ))
    }
}
