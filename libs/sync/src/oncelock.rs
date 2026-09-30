use core::{
    cell::UnsafeCell,
    fmt::Debug,
    hint::spin_loop,
    sync::atomic::{
        self, AtomicU8,
        Ordering::{self, Acquire, Release},
    },
};

/// Calls a method that can only run once. Thread safe

#[derive(Clone, Copy, Debug)]
pub enum OnceState {
    WAITING,
    RUNNING,
    EXECUTED,
    POISONED,
}
impl From<u8> for OnceState {
    fn from(value: u8) -> Self {
        match value {
            0 => OnceState::WAITING,
            1 => OnceState::RUNNING,
            2 => OnceState::EXECUTED,
            _ => OnceState::POISONED,
        }
    }
}

const WAITING: u8 = 0x0;
const RUNNING: u8 = 0x1;
const EXECUTED: u8 = 0x2;
const POISONED: u8 = 0x3;

/// Thread safe implementation of [`OnceCell`] for static globals
pub struct OnceLock<T> {
    state: AtomicU8,
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T: Send + Sync> Sync for OnceLock<T> {}

impl<T> OnceLock<T> {
    pub const fn new() -> Self {
        OnceLock {
            state: AtomicU8::new(WAITING),
            value: UnsafeCell::new(None),
        }
    }

    pub fn set(&self, value: T) {
        if self
            .state
            .compare_exchange(WAITING, RUNNING, Acquire, Acquire)
            .is_ok()
        {
            unsafe {
                *self.value.get() = Some(value);
            }
            self.state.store(EXECUTED, Release);
        } else {
            while self.state.load(Acquire) == RUNNING {
                core::hint::spin_loop();
            }
        }
    }

    pub fn get(&self) -> Option<&T> {
        if self.state.load(Acquire) == EXECUTED {
            // Safe because state is EXECUTED and will never change again
            unsafe { (*self.value.get()).as_ref() }
        } else {
            None
        }
    }
    /// retuns the interior value if this oncelock has been initialized, sets it using `value` otherwise, returning the new value.
    pub fn get_or_set(&self, value: T) -> &T {
        if self
            .state
            .compare_exchange(WAITING, RUNNING, Acquire, Acquire)
            .is_ok()
        {
            unsafe {
                *self.value.get() = Some(value);
                self.state.store(EXECUTED, Release);
            }
        }
        // prob ok to unwrap since it was just set.
        self.get().unwrap()
    }

    /// blocks the thread until the cell is initialized, returning get
    pub fn wait(&self) -> &T {
        while self.state.load(Ordering::Acquire) == WAITING {
            core::hint::spin_loop();
        }
        self.get().unwrap()
    }

    fn get_mut_ptr(&self) -> Option<*mut T> {
        if self.state.load(Acquire) == EXECUTED {
            unsafe { (*self.value.get()).as_mut().map(|v| v as *mut T) }
        } else {
            None
        }
    }
    /// # Safety
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut(&self) -> Option<&mut T> {
        unsafe { self.get_mut_ptr().map(|ptr| &mut *ptr) }
    }
    /// Spins until the cell is initialized
    pub fn wait_mut(&self) -> &mut T {
        loop {
            if self.state.load(Ordering::Acquire) == EXECUTED {
                return unsafe { (*self.value.get()).as_mut().unwrap_unchecked() };
            }
            spin_loop();
        }
    }
}

impl<T> Default for OnceLock<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Debug for OnceLock<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let _ = f.write_str("Once { ");
        if let Some(value) = self.get() {
            let _ = f.write_fmt(format_args!("value: {:?} ", value));
        }
        let _ = f.write_fmt(format_args!(
            "state: {:?} ",
            OnceState::from(self.state.load(Acquire))
        ));
        f.write_str("}")
    }
}
