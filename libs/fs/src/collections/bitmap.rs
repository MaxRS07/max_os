use alloc::vec::Vec;

use crate::{collections::error::FSError, storage::allocator::SectorAllocator};

/// A bitmap-based sector allocator. sector addresses are relative to `start_sector`
#[derive(Debug, Clone)]
pub struct FSBitmap {
    pub start_sector: u64,
    pub size: u64,
    /// Last used cache
    hint: u64,
    map: Vec<u8>,
}

impl FSBitmap {
    pub fn new(start_sector: u64, size: u64) -> Self {
        Self {
            start_sector,
            size,
            map: alloc::vec![0; size.div_ceil(8) as usize],
            hint: start_sector,
        }
    }
    /// Returns `true` if `sector` is used, `false` otherwise
    pub fn is_used(&self, sector: u64) -> bool {
        let Ok(sector) = self.translate_sector(sector) else {
            return false;
        };
        let chunk = sector / 8;
        let bit = sector % 8;
        self.map[chunk as usize] & (1 << bit) != 0
    }
    /// Marks `sector` as `used`
    pub fn set_used(&mut self, sector: u64) -> Result<(), FSError> {
        let sector = self.translate_sector(sector)?;
        if sector > self.size {
            return Err(FSError::OutOfBounds(""));
        }
        let chunk = sector / 8;
        let bit = sector % 8;
        self.map[chunk as usize] |= 1 << bit;
        Ok(())
    }
    pub fn set_free(&mut self, sector: u64) -> Result<(), FSError> {
        let sector = self.translate_sector(sector)?;
        if sector > self.size {
            return Err(FSError::OutOfBounds(""));
        }
        let chunk = sector / 8;
        let bit = sector % 8;
        self.map[chunk as usize] &= !(1 << bit);
        Ok(())
    }
    pub fn set_free_cont(&mut self, sector: u64, len: u64) -> Result<(), FSError> {
        let sector = self.translate_sector(sector)?;
        for i in sector..sector + len {
            self.set_free(i)?;
        }
        self.hint = sector;
        Ok(())
    }
    /// Marks `len` sectors used starting at `sector`
    pub fn set_used_cont(&mut self, sector: u64, len: u64) -> Result<(), FSError> {
        let sector = self.translate_sector(sector)?;
        for i in sector..sector + len {
            self.set_used(i)?;
        }
        Ok(())
    }
    /// Finds and returns the next free sector. Returns `None` if no sectors are free.
    pub fn next_free(&self) -> Option<u64> {
        let mut cur = self.hint;
        let max = self.size;
        while cur < max as u64 {
            if !self.is_used(cur) {
                return Some(cur);
            }
            cur += 1;
        }
        None
    }
    /// Finds and returns the next free sector, marking it used. Returns `None` if no sectors are free.
    pub fn take_free(&mut self) -> Option<u64> {
        let free = self.next_free()?;
        self.set_used(free);
        Some(free)
    }
    /// attempts to fill the buffer with addresses of the next `count` free sectors. If free sectors are exhausted, the remaining addresses are zeroed. Returned sectors are marked used.
    /// Returns the length of filled buffers, 0 if failed.
    pub fn take_free_buf(&mut self, buffer: &mut [u64]) -> Option<()> {
        let mut cur = self.hint;
        for addr in buffer.iter_mut() {
            while cur < self.size {
                if !self.is_used(cur) {
                    *addr = cur;
                    break;
                }
                cur += 1;
            }
            if cur > self.size {
                return None;
            }
        }
        let _ = buffer.iter().map(|i| self.set_used(*i));
        self.hint = cur;
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
        for sector in start..start + len {
            self.set_used(sector);
        }
        Some((start, len))
    }
    /// Retrieves the largest contiguous block of memory of maximum `len` sectors.
    /// # Returns
    /// Tuple (sector, length) where `sector` is the start of the contiguous block and `length` is the size of the block in sectors.
    pub fn largest_cont(&self, max: u64) -> (u64, u64) {
        let mut _start = self.next_free();
        let mut clen = 0;
        let mut max_len = 0;
        let mut max_start = 0;
        while let Some(start) = _start {
            clen += 1;
            while clen < max {
                let sector = start + clen;
                // search overflows, exit and return
                if sector > self.size {
                    _start = None;
                    break;
                }
                if clen == max {
                    return (start, clen);
                }
                // end of contiguous
                if self.is_used(sector) {
                    if clen > max_len {
                        max_len = clen;
                        max_start = start;
                    }
                    _start = self.next_free();
                    clen = 0;
                    break;
                }
            }
        }
        (max_start, max_len)
    }
    /// Translates address `sector` relative to `self.start_sector`
    fn translate_sector(&self, sector: u64) -> Result<u64, FSError> {
        if self.start_sector > sector {
            return Err(FSError::OutOfBounds(
                "Sector address is less than minimum bound",
            ));
        }
        let normal_sector = sector.saturating_sub(self.start_sector);
        if normal_sector > self.size {
            return Err(FSError::OutOfBounds(
                "Sector address exceeds bitmap's bounds",
            ));
        }
        Ok(normal_sector)
    }
}

impl SectorAllocator for FSBitmap {
    fn next_free(&self) -> Option<u64> {
        self.next_free()
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
    fn largest_cont(&self, max: u64) -> (u64, u64) {
        self.largest_cont(max)
    }
}
