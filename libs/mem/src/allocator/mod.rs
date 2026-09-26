use core::alloc::{GlobalAlloc, Layout};

use crate::error::MemoryError;

pub mod bump;
pub mod linked_list;

/// Custom trait for primitive heap allocators. This allows error prop to k space and locking for global allocator
trait HeapAllocator {
    fn alloc(&mut self, layout: Layout) -> Result<*mut u8, MemoryError>;
    fn free(&mut self, ptr: *mut u8, layout: Layout) -> Result<(), MemoryError>;
}
