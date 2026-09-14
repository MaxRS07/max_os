use crate::collections::error::FSError;

pub trait SectorAllocator {
    fn next_free(&self) -> Option<u64>;
    fn take_free(&mut self) -> Option<u64>;
    fn take_free_buf(&mut self, buffer: &mut [u64]) -> Option<()>;
    fn largest_cont(&self, max: u64) -> (u64, u64);
    fn take_largest_cont(&mut self, size: u64) -> Option<(u64, u64)>;
    /// Marks up to `max` contiguous free sectors starting exactly at `start` as used,
    /// returning how many were taken. Used to grow an extent in place.
    fn take_cont_at(&mut self, start: u64, max: u64) -> u64;
    /// `true` when `sector` is allocated or outside this allocator's range.
    fn is_used(&self, sector: u64) -> bool;
    fn set_used(&mut self, sector: u64) -> Result<(), FSError>;
    fn set_used_cont(&mut self, start: u64, size: u64) -> Result<(), FSError>;
    fn set_free(&mut self, sector: u64) -> Result<(), FSError>;
    fn set_free_cont(&mut self, sector: u64, len: u64) -> Result<(), FSError>;
}
