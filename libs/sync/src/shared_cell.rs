use core::cell::UnsafeCell;

/// A simple wrapper to force Sync for mutable cells
/// TODO: fix and make atomic
pub struct SharedCell<T>(UnsafeCell<T>)
where
    T: Sized;

unsafe impl<T> Sync for SharedCell<T> {}

impl<T> SharedCell<T> {
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    /// Access the inner device mutably
    /// # Safety
    /// caller must ensure interrupts are disabled or CPU execution is exclusive.
    pub unsafe fn get_mut<'a>(&self) -> &'a mut T {
        unsafe { &mut *self.0.get() }
    }
}
