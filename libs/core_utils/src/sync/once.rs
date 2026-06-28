use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicU8, Ordering},
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

pub struct Once<T> {
    state: AtomicU8,
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T: Send + Sync> Sync for Once<T> {}

impl<T> Once<T> {
    pub const fn new() -> Self {
        Once {
            state: AtomicU8::new(WAITING),
            value: UnsafeCell::new(None),
        }
    }

    pub fn init<F>(&self, func: F)
    where
        F: FnOnce() -> T,
    {
        match self
            .state
            .compare_exchange(WAITING, RUNNING, Ordering::Acquire, Ordering::Acquire)
        {
            Ok(WAITING) => {
                unsafe {
                    *self.value.get() = Some(func());
                }
                self.state.store(EXECUTED, Ordering::Release);
            }
            Err(RUNNING) => {
                while self.state.load(Ordering::Acquire) == RUNNING {
                    core::hint::spin_loop();
                }
            }
            Err(EXECUTED) | Err(POISONED) => {
                panic!("Once already init, state={:?}", self.state());
            }
            _ => unreachable!(),
        }
    }
    pub fn get(&self) -> Option<&T> {
        if self.state.load(Ordering::Acquire) == EXECUTED {
            // Safe because state is EXECUTED and will never change again
            unsafe { (*self.value.get()).as_ref() }
        } else {
            None
        }
    }
    pub fn get_mut(&self) -> Option<*mut T> {
        if self.state.load(Ordering::Acquire) == EXECUTED {
            unsafe { (*self.value.get()).as_mut().map(|v| v as *mut T) }
        } else {
            None
        }
    }
    pub fn is_waiting(&self) -> bool {
        self.state.load(Ordering::Acquire) == WAITING
    }
    pub fn is_executed(&self) -> bool {
        self.state.load(Ordering::Acquire) == EXECUTED
    }
    pub fn is_poisoned(&self) -> bool {
        self.state.load(Ordering::Acquire) == POISONED
    }
    pub fn is_running(&self) -> bool {
        self.state.load(Ordering::Acquire) == RUNNING
    }
    pub fn state(&self) -> OnceState {
        OnceState::from(self.state.load(Ordering::Acquire))
    }
}

impl<T> Default for Once<T> {
    fn default() -> Self {
        Self::new()
    }
}
