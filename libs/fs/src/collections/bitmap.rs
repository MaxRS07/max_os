use alloc::{borrow::ToOwned, vec::Vec};

use crate::{collections::error::FSError, storage::allocator::SectorAllocator};

/// A bitmap-based sector allocator. sector addresses are relative to `start_sector`
#[derive(Debug, Clone)]
pub struct FSBitmap {
    pub start_sector: u64,
    pub size: u64,
    /// Index (bitmap-relative, *not* a sector address) of the lowest sector that may still be free.
    hint: u64,
    map: Vec<u8>,
}

impl FSBitmap {
    pub fn new(start_sector: u64, size: u64) -> Self {
        Self {
            start_sector,
            size,
            map: alloc::vec![0; size.div_ceil(8) as usize],
            hint: 0,
        }
    }
    /// Returns `true` if `sector` is used. Addresses outside the map read as used so that
    /// allocation scans never hand out a sector this bitmap does not own.
    pub fn is_used(&self, sector: u64) -> bool {
        let Ok(idx) = self.translate_sector(sector) else {
            return true;
        };
        self.used_idx(idx)
    }
    /// Marks `sector` as `used`
    pub fn set_used(&mut self, sector: u64) -> Result<(), FSError> {
        let idx = self.translate_sector(sector)?;
        self.set_used_idx(idx);
        Ok(())
    }
    pub fn set_free(&mut self, sector: u64) -> Result<(), FSError> {
        let idx = self.translate_sector(sector)?;
        self.set_free_idx(idx);
        Ok(())
    }
    pub fn set_free_cont(&mut self, sector: u64, len: u64) -> Result<(), FSError> {
        let idx = self.translate_sector(sector)?;
        self.check_run(idx, len)?;
        for i in idx..idx + len {
            self.set_free_idx(i);
        }
        Ok(())
    }
    /// Marks `len` sectors used starting at `sector`
    pub fn set_used_cont(&mut self, sector: u64, len: u64) -> Result<(), FSError> {
        let idx = self.translate_sector(sector)?;
        self.check_run(idx, len)?;
        for i in idx..idx + len {
            self.set_used_idx(i);
        }
        Ok(())
    }
    /// Finds and returns the next free sector. Returns `None` if no sectors are free.
    pub fn next_free(&self) -> Option<u64> {
        let mut idx = self.hint;
        while idx < self.size {
            if !self.used_idx(idx) {
                return Some(self.start_sector + idx);
            }
            idx += 1;
        }
        None
    }
    /// Finds and returns the next free sector, marking it used. Returns `None` if no sectors are free.
    pub fn take_free(&mut self) -> Option<u64> {
        let free = self.next_free()?;
        self.set_used(free).ok()?;
        Some(free)
    }
    /// Fills `buffer` with the addresses of the next `buffer.len()` free sectors, marking each used.
    /// Returns `None` and leaves the bitmap untouched if there are not enough free sectors.
    pub fn take_free_buf(&mut self, buffer: &mut [u64]) -> Option<()> {
        let mut idx = self.hint;
        let mut taken = 0;
        for slot in buffer.iter_mut() {
            while idx < self.size && self.used_idx(idx) {
                idx += 1;
            }
            if idx >= self.size {
                // roll back so a failed allocation does not leak sectors
                for addr in &buffer[..taken] {
                    let _ = self.set_free(*addr);
                }
                return None;
            }
            *slot = self.start_sector + idx;
            self.set_used_idx(idx);
            taken += 1;
            idx += 1;
        }
        Some(())
    }
    /// Retrieves the largest contiguous block of memory of maximum `len` sectors, marking all sectors used
    /// # Returns
    /// Tuple (sector, length) where `sector` is the start of the contiguous block and `length` is the size of the block in sectors.
    pub fn take_largest_cont(&mut self, len: u64) -> Option<(u64, u64)> {
        let (start, len) = self.largest_cont(len);
        if len == 0 {
            return None;
        }
        self.set_used_cont(start, len).ok()?;
        Some((start, len))
    }
    /// Marks up to `max` contiguous free sectors starting exactly at `start` as used.
    /// Returns how many sectors were taken, which may be 0.
    pub fn take_cont_at(&mut self, start: u64, max: u64) -> u64 {
        let Ok(idx) = self.translate_sector(start) else {
            return 0;
        };
        let mut taken = 0;
        while taken < max && idx + taken < self.size && !self.used_idx(idx + taken) {
            self.set_used_idx(idx + taken);
            taken += 1;
        }
        taken
    }
    /// Retrieves the largest contiguous block of memory of maximum `max` sectors.
    /// Stops early once a run of exactly `max` sectors is found.
    /// # Returns
    /// Tuple (sector, length) where `sector` is the start of the contiguous block and `length` is the size of the block in sectors.
    pub fn largest_cont(&self, max: u64) -> (u64, u64) {
        if max == 0 {
            return (0, 0);
        }
        let mut best_start = 0;
        let mut best_len = 0;
        let mut idx = self.hint;

        while idx < self.size {
            if self.used_idx(idx) {
                idx += 1;
                continue;
            }
            let run_start = idx;
            let mut run_len = 0;
            while idx < self.size && run_len < max && !self.used_idx(idx) {
                idx += 1;
                run_len += 1;
            }
            if run_len > best_len {
                best_len = run_len;
                best_start = run_start;
                // cannot do better than `max`
                if best_len == max {
                    break;
                }
            }
        }
        if best_len == 0 {
            return (0, 0);
        }
        (self.start_sector + best_start, best_len)
    }

    /* index-space helpers, all take a bitmap-relative index */

    fn used_idx(&self, idx: u64) -> bool {
        if idx >= self.size {
            return true;
        }
        self.map[(idx / 8) as usize] & (1 << (idx % 8)) != 0
    }
    fn set_used_idx(&mut self, idx: u64) {
        if idx >= self.size {
            return;
        }
        self.map[(idx / 8) as usize] |= 1 << (idx % 8);
        if idx == self.hint {
            self.hint = idx + 1;
        }
    }
    fn set_free_idx(&mut self, idx: u64) {
        if idx >= self.size {
            return;
        }
        self.map[(idx / 8) as usize] &= !(1 << (idx % 8));
        if idx < self.hint {
            self.hint = idx;
        }
    }
    fn check_run(&self, idx: u64, len: u64) -> Result<(), FSError> {
        if idx + len > self.size {
            return Err(FSError::OutOfBounds(
                "Sector run exceeds bitmap's bounds".to_owned(),
            ));
        }
        Ok(())
    }
    /// Translates address `sector` relative to `self.start_sector`
    fn translate_sector(&self, sector: u64) -> Result<u64, FSError> {
        if self.start_sector > sector {
            return Err(FSError::OutOfBounds(
                "Sector address is less than minimum bound".to_owned(),
            ));
        }
        let normal_sector = sector - self.start_sector;
        if normal_sector >= self.size {
            return Err(FSError::OutOfBounds(
                "Sector address exceeds bitmap's bounds".to_owned(),
            ));
        }
        Ok(normal_sector)
    }
}

impl SectorAllocator for FSBitmap {
    fn next_free(&self) -> Option<u64> {
        self.next_free()
    }
    fn is_used(&self, sector: u64) -> bool {
        self.is_used(sector)
    }
    fn set_free(&mut self, sector: u64) -> core::result::Result<(), FSError> {
        self.set_free(sector)
    }
    fn set_free_cont(&mut self, sector: u64, len: u64) -> core::result::Result<(), FSError> {
        self.set_free_cont(sector, len)
    }
    fn set_used(&mut self, sector: u64) -> Result<(), super::error::FSError> {
        self.set_used(sector)
    }
    fn set_used_cont(&mut self, start: u64, size: u64) -> core::result::Result<(), FSError> {
        self.set_used_cont(start, size)
    }
    fn take_free(&mut self) -> Option<u64> {
        self.take_free()
    }
    fn take_free_buf(&mut self, buffer: &mut [u64]) -> Option<()> {
        self.take_free_buf(buffer)
    }
    fn take_largest_cont(&mut self, size: u64) -> Option<(u64, u64)> {
        self.take_largest_cont(size)
    }
    fn take_cont_at(&mut self, start: u64, max: u64) -> u64 {
        self.take_cont_at(start, max)
    }
    fn largest_cont(&self, max: u64) -> (u64, u64) {
        self.largest_cont(max)
    }
}
