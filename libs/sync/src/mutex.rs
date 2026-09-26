use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Mutex<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

unsafe impl<T> Send for Mutex<T> {}
unsafe impl<T> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }
    #[allow(clippy::mut_from_ref)]
    /// attempts to acquire a lock, returning [`Some`] containing a mutable ref to the inner value if successful. returns [`None`] otherwise.
    pub fn try_lock(&self) -> Option<&mut T> {
        self.locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| unsafe { &mut *self.value.get() })
    }
    /// spins until a lock is accuired, returning a mutable ref to the interior value T
    #[allow(clippy::mut_from_ref)]
    pub fn lock(&self) -> &mut T {
        loop {
            match self.try_lock() {
                Some(value) => return value,
                None => core::hint::spin_loop(),
            }
        }
    }
}
