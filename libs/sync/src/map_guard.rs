use core::ops::{Deref, DerefMut};

use crate::{mutex::Mutex, mutex_guard::MutexGuard};

pub struct MappedMutexGuard<'a, T, U> {
    mutex: &'a Mutex<T>,
    value: *mut U,
}

impl<'a, T, U> MappedMutexGuard<'a, T, U> {
    pub fn new(mutex: &'a Mutex<T>, value: &mut U) -> Self {
        Self { mutex, value }
    }
}

impl<'a, T, U> Deref for MappedMutexGuard<'a, T, U> {
    type Target = U;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.value }
    }
}
impl<'a, T, U> DerefMut for MappedMutexGuard<'a, T, U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.value }
    }
}

impl<'a, T, U> Drop for MappedMutexGuard<'a, T, U> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}
