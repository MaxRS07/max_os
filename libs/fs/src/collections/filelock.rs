use core::{
    hash::Hash,
    ptr::write_volatile,
    sync::atomic::{AtomicU8, AtomicU32, Ordering},
};

use alloc::{format, sync::Arc};
use collections::hashmap::HashMap;

use crate::{
    collections::error::FSError,
    meta::{
        inode,
        lockstate::LockState::{self, Exclusive},
    },
    storage::sector::FSInodeSector,
};

/// Inode locking map

#[derive(Debug)]
pub struct LockStateData {
    state: AtomicU8,
    refs: AtomicU32,
}

impl LockStateData {
    pub fn new() -> Self {
        let unlocked = LockState::Unlocked as u8;
        Self {
            state: AtomicU8::new(unlocked),
            refs: AtomicU32::new(0),
        }
    }
    pub fn lock_state(&self) -> LockState {
        LockState::from(self.state.load(Ordering::Acquire))
    }
    pub fn set_state(&self, state: LockState) {
        self.state.store(state as u8, Ordering::Release);
    }
    pub fn refs(&self) -> u32 {
        self.refs.load(Ordering::Acquire)
    }
    pub fn clear_refs(&self) {
        self.refs.store(0, Ordering::Release);
        self.set_state(LockState::Unlocked);
    }
    fn add_ref(&self) {
        self.refs.fetch_add(1, Ordering::Release);
    }
    fn remove_ref(&self) {
        let num_refs = self.refs.fetch_sub(1, Ordering::Release);
        if num_refs == 1 {
            core::sync::atomic::fence(Ordering::Acquire);
            self.set_state(LockState::Unlocked);
        }
    }
    pub fn try_lock_shared(&self) -> Result<(), FSError> {
        loop {
            let current = self.state.load(Ordering::Acquire);
            if current == LockState::Exclusive as u8 {
                return Err(FSError::IOError("Failed to acquire lock"));
            }
            match self.state.compare_exchange_weak(
                current,
                LockState::Shared as u8,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.add_ref();
                    return Ok(());
                }
                Err(_) => core::hint::spin_loop(),
            }
        }
    }
    pub fn lock_shared(&self) {
        loop {
            // wait until exlusive is released
            while self.state.load(Ordering::Acquire) == LockState::Exclusive as u8 {
                core::hint::spin_loop();
            }
            if self.try_lock_shared().is_ok() {
                return;
            }
            core::hint::spin_loop();
        }
    }
    pub fn release_shared(&self) {
        self.remove_ref();
    }
    pub fn try_lock_exclusive(&self) -> Result<(), FSError> {
        let res = self.state.compare_exchange(
            LockState::Unlocked as u8,
            LockState::Exclusive as u8,
            Ordering::Acquire,
            Ordering::Relaxed,
        );

        if res.is_err() {
            return Err(FSError::IOError("Lock is currently held"));
        }

        if self.refs() != 0 {
            self.state
                .store(LockState::Unlocked as u8, Ordering::Release);
            return Err(FSError::IOError("Lock has active readers"));
        }

        Ok(())
    }
    pub fn lock_exclusive(&self) {
        loop {
            while self.state.load(Ordering::Acquire) != LockState::Unlocked as u8
                || self.refs() != 0
            {
                core::hint::spin_loop();
            }
            if self.try_lock_exclusive().is_ok() {
                return;
            }
            core::hint::spin_loop();
        }
    }
    pub fn release_exclusive(&self) {
        loop {
            let res = self.state.compare_exchange(
                LockState::Exclusive as u8,
                LockState::Unlocked as u8,
                Ordering::Acquire,
                Ordering::Relaxed,
            );
            if res.is_ok() {
                return;
            }
            core::hint::spin_loop();
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct LockTable {
    map: HashMap<u64, Arc<LockStateData>>,
}
impl LockTable {
    /// gets the file lock associated with `inode_sector` if present. Otherwise a new lock state is created.
    pub fn get_lock(&mut self, inode_sector: u64) -> Option<Arc<LockStateData>> {
        if let Some(lock_data) = self.map.get(&inode_sector) {
            return Some(Arc::clone(&lock_data));
        }

        let new_state = Arc::new(LockStateData::new());
        self.map.insert(inode_sector, Arc::clone(&new_state));
        Some(new_state)
    }
}
