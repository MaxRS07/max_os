use core::{
    cell::UnsafeCell,
    fmt::Debug,
    sync::atomic::{AtomicBool, Ordering},
};

use time::global_time::GlobalTimer;

use crate::mutex_guard::MutexGuard;

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
    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        self.locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| MutexGuard::new(&self))
    }
    /// spins until a lock is accuired, returning a mutable ref to the interior value T
    #[allow(clippy::mut_from_ref)]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        loop {
            match self.try_lock() {
                Some(value) => return value,
                None => core::hint::spin_loop(),
            }
        }
    }
    /// attempts to lock for
    pub fn lock_timeout(
        &self,
        timer: &dyn GlobalTimer,
        max_time: u64,
    ) -> Result<MutexGuard<'_, T>, &'static str> {
        let start = timer.now_ms();
        loop {
            match self.try_lock() {
                Some(value) => return Ok(value),
                None => {
                    core::hint::spin_loop();
                }
            }
            if timer.elapsed(start, max_time) {
                return Err("Failed to get instace, lock timeout");
            }
        }
    }
    /// unlocks this mutex
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
    pub fn value_ptr(&self) -> *mut T {
        self.value.get()
    }
    pub fn get(&self) -> &T {
        unsafe { &*self.value.get() }
    }
}

impl<T> AsRef<T> for Mutex<T> {
    fn as_ref(&self) -> &T {
        unsafe { &*self.value.get() }
    }
}

impl<T> Debug for Mutex<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Mutex")
            .field("locked", &self.locked.load(Ordering::Acquire))
            .field("value", unsafe { &*self.value.get() })
            .finish()
    }
}
