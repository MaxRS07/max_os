use core::ops::{Deref, DerefMut};

use crate::{map_guard::MappedMutexGuard, mutex::Mutex};

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}
impl<'a, T> MutexGuard<'a, T> {
    pub fn new(mutex: &'a Mutex<T>) -> Self {
        Self { mutex }
    }
    pub fn map<U, F>(self, f: F) -> MappedMutexGuard<'a, T, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        let mutex = self.mutex;
        let value = f(unsafe { &mut *mutex.value_ptr() });
        core::mem::forget(self);
        MappedMutexGuard::new(mutex, value)
    }
    pub fn try_map<U, F, E>(self, f: F) -> Result<MappedMutexGuard<'a, T, U>, E>
    where
        F: FnOnce(&mut T) -> Result<&mut U, E>,
    {
        let mutex = self.mutex;
        let value = f(unsafe { &mut *mutex.value_ptr() })?;
        core::mem::forget(self);
        Ok(MappedMutexGuard::new(mutex, value))
    }
}
impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value_ptr() }
    }
}
impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value_ptr() }
    }
}
impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}
