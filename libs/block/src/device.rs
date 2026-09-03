use alloc::vec::Vec;

use crate::error::BlockError;

/// block device trait for bulk read write functions
pub trait BlockDevice: Send + Sync {
    /// Size of an individual sector in bytes
    fn sector_size(&self) -> u32;
    /// Total sectors available from device
    fn capacity(&self) -> u64;

    /// Reads a single sector and returns a `sector_size()` length `Vec<u8>`
    fn read_sector(&mut self, sector: u64) -> Result<Vec<u8>, BlockError>;

    /// Reads a single sector and fills a `buffer.len()` length byte array
    fn read_buffer(&mut self, sector: u64, buffer: &mut [u8]) -> Result<usize, BlockError>;

    /// Writes a byte array directly to a sector at an offset
    fn write_buffer(&mut self, sector: u64, offset: u64, data: &[u8]) -> Result<(), BlockError>;

    /// Writes a byte array directly to a sector at an offset. Does not write if `data.len() + offset` exceeds `sector_size()`
    fn write_sector_offset(
        &mut self,
        sector: u64,
        data: &[u8],
        offset: u64,
    ) -> Result<(), BlockError>;
}
