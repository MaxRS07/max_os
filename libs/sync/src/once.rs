use core::{
    cell::UnsafeCell,
    fmt::Debug,
    sync::atomic::{
        self, AtomicUsize,
        Ordering::{Acquire, Release},
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
impl From<usize> for OnceState {
    fn from(value: usize) -> Self {
        match value {
            0 => OnceState::WAITING,
            1 => OnceState::RUNNING,
            2 => OnceState::EXECUTED,
            _ => OnceState::POISONED,
        }
    }
}

const WAITING: usize = 0x0;
const RUNNING: usize = 0x1;
const EXECUTED: usize = 0x2;
const POISONED: usize = 0x3;

/// Thread safe implementation of [`OnceCell`] for mutable statics
pub struct OnceLock<T> {
    state: AtomicUsize,
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T: Send + Sync> Sync for OnceLock<T> {}

impl<T> OnceLock<T> {
    pub const fn new() -> Self {
        OnceLock {
            state: AtomicUsize::new(WAITING),
            value: UnsafeCell::new(None),
        }
    }

    pub fn init<F>(&self, func: F)
    where
        F: FnOnce() -> T,
    {
        if self
            .state
            .compare_exchange(WAITING, RUNNING, Acquire, Acquire)
            .is_ok()
        {
            unsafe {
                *self.value.get() = Some(func());
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
    pub fn get_mut_ptr(&self) -> Option<*mut T> {
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
    pub fn is_waiting(&self) -> bool {
        self.state.load(Acquire) == WAITING
    }
    pub fn is_executed(&self) -> bool {
        self.state.load(Acquire) == EXECUTED
    }
    pub fn is_poisoned(&self) -> bool {
        self.state.load(Acquire) == POISONED
    }
    pub fn is_running(&self) -> bool {
        self.state.load(Acquire) == RUNNING
    }
    pub fn state(&self) -> OnceState {
        OnceState::from(self.state.load(Acquire))
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
        f.write_str("Once { ");
        if let Some(value) = self.get() {
            f.write_fmt(format_args!("value: {:?} ", value));
        }
        f.write_fmt(format_args!(
            "state: {:?} ",
            OnceState::from(self.state.load(Acquire))
        ));
        f.write_str("}")
    }
}
