extern crate alloc;

use core::alloc::Layout;

use crate::sync::once::Once;

pub struct Box<T> {
    ptr: *mut T,
}
impl<T> Box<T> {
    pub fn new(val: T) -> Box<T> {
        if let Ok(layout) = Layout::from_size_align(size_of::<T>(), align_of::<T>()) {
            unsafe {
                let ptr = alloc::alloc::alloc(layout);
                let t_ptr = ptr as *mut T;
                t_ptr.write_volatile(val);
                Self { ptr: t_ptr }
            }
        } else {
            panic!("GlobalAlloc not yet implemented")
        }
    }
    pub fn addr(&self) -> usize {
        self.ptr as usize
    }
    pub fn get_mut(&self) -> &'static mut T {
        unsafe { self.ptr.as_mut().unwrap() }
    }
}
