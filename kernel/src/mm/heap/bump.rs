/// Simple bump allocator for boot instructions, before ram
use crate::mm::error::MemoryError;
use crate::mm::heap::{_end, BOOT_HEAP_SIZE};

static mut ALLOC_PTR: usize = 0;

pub fn init() {
    unsafe {
        ALLOC_PTR = (_end as *const u8).addr();
    }
}
/// allocates `size * core::mem::size_of::<T>()` in the bump. Aligned to 8 bytes
pub fn bump_alloc<T>(size: usize) -> Result<*mut T, MemoryError> {
    let size = (core::mem::size_of::<T>() * size + 7) & !7;
    let max_heap = unsafe { _end + BOOT_HEAP_SIZE };
    if unsafe { ALLOC_PTR + size > max_heap } {
        return Err(MemoryError::OutOfMemory("bump is limited to 4KiB"));
    }
    unsafe {
        validate_bump_addr();
        let ptr = ALLOC_PTR as *mut T;
        ALLOC_PTR += size;
        Ok(ptr)
    }
}
/// Panics if bump has not been initialized
pub fn validate_bump_addr() {
    unsafe {
        if ALLOC_PTR == 0 {
            panic!(
                "ALLOC_PTR has not been initialized, you must call 'ptr_init' before allocation"
            );
        }
    }
}
