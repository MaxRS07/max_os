use core::{
    ops::Index,
    sync::atomic::{AtomicU32, Ordering},
};

use alloc::{format, vec::Vec};
use block::device::BlockDevice;

use crate::{
    collections::error::FSError,
    core::{locator::FSLocator, mutpath, path::FSPath},
    storage::volume::FSVolume,
};

pub struct FSMountEntry<'a> {
    id: u32,
    pub prefix: &'a FSPath,
    pub volume: FSVolume<'a>,
}

impl<'a> FSMountEntry<'a> {
    pub fn new(id: u32, prefix: &'a FSPath, volume: FSVolume<'a>) -> Self {
        Self { id, prefix, volume }
    }
}
impl<'a> PartialEq for FSMountEntry<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct FSMountTable<'a> {
    table: Vec<FSMountEntry<'a>>,
    next_id: AtomicU32,
}

impl<'a> FSMountTable<'a> {
    pub fn new() -> Self {
        Self {
            table: alloc::vec![],
            next_id: AtomicU32::new(0),
        }
    }
    pub fn mount(
        &mut self,
        path: &'a FSPath,
        blk_dev: &'a mut dyn BlockDevice,
    ) -> Result<u32, FSError> {
        let volume = FSVolume::from_block_device(blk_dev)?;
        let id = self.next_id.fetch_add(1, Ordering::AcqRel);
        let entry = FSMountEntry::new(id, path, volume);
        self.table.push(entry);
        Ok(id)
    }
    pub fn unmount(&mut self, path: &'a FSPath) -> Option<FSVolume<'a>> {
        if let Some(idx) = self.table.iter().position(|e| e.prefix == path) {
            return Some(self.table.swap_remove(idx).volume);
        }
        None
    }
    /// Returns the `FSVolume` with a matching prefix if present, otherwise `None`
    pub fn find_volume(&mut self, path: &FSPath) -> Option<&mut FSVolume<'a>> {
        if !path.is_absolute() {
            return None;
        }
        for entry in self.table.iter_mut() {
            if entry.prefix.matches_prefix(path) {

                return Some(&mut entry.volume);
            }
        }
        None
    }
}

impl<'a> Default for FSMountTable<'a> {
    fn default() -> Self {
        Self::new()
    }
}
