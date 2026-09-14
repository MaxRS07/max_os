use core::ptr::addr_of;

use alloc::format;
use block::device::BlockDevice;

use crate::{
    collections::error::FSError,
    meta::{
        header::FSHeader,
        inode::{BLOCK_SIZE, FSInode, as_bytes_mut, block_to_sector},
    },
    storage::sector::{self, FSHeaderSector, FSInodeSector},
};
/// Block device wrapper for FS functions
pub struct FSBlockStore<'a> {
    blk_dev: &'a mut dyn BlockDevice,
}

impl<'a> FSBlockStore<'a> {
    pub fn new(blk_dev: &'a mut dyn BlockDevice) -> Self {
        Self { blk_dev }
    }
    pub fn capacity(&self) -> u64 {
        self.blk_dev.capacity()
    }
    /// Raw device behind this store, for the few paths that still need buffer-level access.
    pub fn device(&mut self) -> &mut dyn BlockDevice {
        self.blk_dev
    }
    /// Overwrites logical block `block` (4KiB) with zeroes.
    pub fn zero_block(&mut self, block: u64) -> Result<(), FSError> {
        self.write_buffer(block_to_sector(block), 0, &[0u8; BLOCK_SIZE as usize])
    }
    /// Reads a `FSHeader` from disk
    pub fn read_header(&mut self, address: FSHeaderSector) -> Result<FSHeader, FSError> {
        let sector = address.sector();
        let offset = address.offset();
        self.read(sector, offset)
    }
    pub fn write_buffer(&mut self, sector: u64, offset: u64, buffer: &[u8]) -> Result<(), FSError> {
        self.blk_dev
            .write_buffer(sector, offset, buffer)
            .map_err(|e| e.into())
    }
    /// Writes an `FSHeader` to disk.
    pub fn write_header(
        &mut self,
        address: FSHeaderSector,
        header: FSHeader,
    ) -> Result<(), FSError> {
        let sector = address.sector();
        let offset = address.offset();
        self.write(sector, offset, header)
    }
    /// Reads an `FSInode` to disk
    pub fn read_inode(&mut self, address: FSInodeSector) -> Result<FSInode, FSError> {
        let sector = address.sector();
        let offset = address.offset();
        self.read(sector, offset)
    }
    /// Write an `FSInode` to disk
    pub fn write_inode(&mut self, address: FSInodeSector, inode: FSInode) -> Result<(), FSError> {
        let sector = address.sector();
        let offset = address.offset();
        self.write(sector, offset, inode)
    }
    /// Reads a full indirect-pointer block (`ADDRS_PER_BLOCK` `u64` entries) at logical
    /// block `block` into `buf`.
    pub fn read_index_block(&mut self, block: u64, buf: &mut [u64]) -> Result<(), FSError> {
        self.blk_dev
            .read_buffer(block_to_sector(block), as_bytes_mut(buf))?;
        Ok(())
    }
    /// Writes data of type `T` to a specific sub-sector offset within a sector.
    /// `T` must implement `Copy` to prevent dropping/double-free issues.
    pub fn write<T>(&mut self, sector: u64, offset: u64, value: T) -> Result<(), FSError> {
        let align = align_of::<T>() as u64;
        let size = size_of::<T>() as u64;
        let sector_size = self.blk_dev.sector_size() as u64;

        // must fit within a single sector
        if offset + size > sector_size {
            return Err(FSError::Type(format!(
                "Write failed: T overflows sector boundary"
            )));
        }
        // align to T
        if !offset.is_multiple_of(align) {
            return Err(FSError::Type(format!(
                "Write failed: offset {offset} not aligned to T alignment {align}"
            )));
        }
        let byte_ptr = addr_of!(value) as *const u8;
        let bytes = unsafe { core::slice::from_raw_parts(byte_ptr, size as usize) };

        self.blk_dev.write_sector_offset(sector, bytes, offset)?;
        Ok(())
    }

    /// Reads data of type `T` from a specific sub-sector offset within a sector.
    pub fn read<T>(&mut self, sector: u64, offset: u64) -> Result<T, FSError>
    where
        T: Copy,
    {
        let align = align_of::<T>() as u64;
        let size = size_of::<T>() as u64;
        let sector_size = self.blk_dev.sector_size() as u64;

        if offset + size > sector_size {
            return Err(FSError::Type(format!(
                "Read failed: T (size {size}) at offset {offset} overflows sector"
            )));
        }
        if !offset.is_multiple_of(align) {
            return Err(FSError::Type(format!(
                "Read failed: offset {offset} not aligned to T alignment {align}"
            )));
        }

        let mut buffer = alloc::vec![0u8; sector_size as usize];
        self.blk_dev.read_buffer(sector, &mut buffer)?;

        unsafe {
            let byte_ptr = buffer.as_ptr().add(offset as usize) as *const T;
            Ok(byte_ptr.read_unaligned())
        }
    }
    /// Attempts to fill `buffer` starting at index `start`. Returns the len of bytes written on success.
    pub fn read_to_buffer(
        &mut self,
        sector: u64,
        buffer: &mut [u8],
        start: usize,
    ) -> Result<usize, FSError> {
        self.blk_dev
            .read_buffer(sector, &mut buffer[start..])
            .map_err(|e| e.into())
    }
}
