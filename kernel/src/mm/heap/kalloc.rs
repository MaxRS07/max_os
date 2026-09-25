use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::{null, null_mut},
    sync::atomic::Ordering,
};

use log::{debug, error, info};

use crate::mm::{
    error::MemoryError,
    heap::{KERNEL_ALLOCATOR, PAGE_ALLOCATOR, page_allocator},
};
