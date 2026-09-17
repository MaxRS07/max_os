use core::{
    cmp::min,
    ops::Range,
    ptr::{addr_of, addr_of_mut},
};

use alloc::vec::Vec;
use block::{device::BlockDevice, error::BlockError};
use log::{error, warn};
use mmio::mmio_read;
use sdt::fdt::FDT;
use virtio::{
    ALL_FEATURES,
    init::{init_state, signal_driver_ok},
    queue::init_virtqueue,
    types::{
        device::verify_virtio_magic,
        queue::{DescBuf, VirtQueue},
    },
};

use crate::drivers::block::virtio::cfg::{
    VIRTIO_BLK_CFG_OFFSET, VIRTIO_BLK_S_IOERR, VIRTIO_BLK_S_OK, VIRTIO_BLK_T_IN, VIRTIO_BLK_T_OUT,
    VirtioBlkConfig, VirtioBlkOutHeader,
};

pub mod cfg;
pub const VIRTIO_BLOCK_ID: u32 = 2;

/// Maximum size of any single segment is in size_max.
pub const VIRTIO_BLK_F_SIZE_MAX: u64 = 1 << 1;
/// Maximum number of segments in a request is in seg_max.
pub const VIRTIO_BLK_F_SEG_MAX: u64 = 1 << 2;
/// Disk-style geometry specified in geometry.
pub const VIRTIO_BLK_F_GEOMETRY: u64 = 1 << 4;
/// Device is read-only.
pub const VIRTIO_BLK_F_RO: u64 = 1 << 5;
/// Block size of disk is in blk_size.
pub const VIRTIO_BLK_F_BLK_SIZE: u64 = 1 << 6;
/// Cache flush command support.
pub const VIRTIO_BLK_F_FLUSH: u64 = 1 << 9;
/// Device exports information on optimal I/O alignment.
pub const VIRTIO_BLK_F_TOPOLOGY: u64 = 1 << 10;
/// Device can toggle its cache between writeback and writethrough modes.
pub const VIRTIO_BLK_F_CONFIG_WCE: u64 = 1 << 11;
/// Device can support discard command, maximum discard sectors size in max_discard_sectors and maximum discard segment number in max_discard_seg.
pub const VIRTIO_BLK_F_DISCARD: u64 = 1 << 13;
/// Device can support write zeroes command, maximum write zeroes sectors size in max_write_zeroes_sectors and maximum write zeroes segment number in max_write_zeroes_seg.
pub const VIRTIO_BLK_F_WRITE_ZEROES: u64 = 1 << 14;

// LEGACY INTERFACE

/// Device supports scsi packet commands.
pub const VIRTIO_BLK_F_SCSI: u64 = 1 << 7;

pub struct VirtioBlock {
    mmio_addr: usize,
    /// queues for operation requests, not necissarily in order according the the spec
    requestq: VirtQueue,
    pub cfg: VirtioBlkConfig,
}

impl VirtioBlock {
    pub fn from_mmio(fdt: &FDT, mmio_idx: usize) -> Option<Self> {
        let mmio_addr = fdt.virtio_mmio[mmio_idx].base_address;
        if let Err(error) = verify_virtio_magic(mmio_addr) {
            warn!("{}", error);
            return None;
        }
        // select all features before reading config. Use all features except RO
        if let Err(error) = init_state(mmio_addr, ALL_FEATURES & !VIRTIO_BLK_F_RO) {
            warn!("{}", error);
            return None;
        }

        let mut requestq = VirtQueue::default();
        let cfg: VirtioBlkConfig = mmio::mmio_read(mmio_addr, VIRTIO_BLK_CFG_OFFSET);

        if let Err(error) = init_virtqueue(&mut requestq, mmio_addr, 0) {
            error!("Failed to initialize block request queue: {}", error);
            return None;
        }
        signal_driver_ok(mmio_addr);

        let blk = VirtioBlock {
            mmio_addr,
            requestq,
            cfg,
        };

        Some(blk)
    }
    pub fn blk_read(&mut self, sector: u64, len: u32) -> Result<Vec<u8>, BlockError> {
        let hdr = VirtioBlkOutHeader {
            _type: VIRTIO_BLK_T_IN,
            reserved: 0,
            sector,
        };
        let mut status = 0xFFu8;
        let mut data = alloc::vec![0u8; len as usize];

        let bufs = [
            DescBuf {
                addr: core::ptr::addr_of!(hdr) as u64,
                len: core::mem::size_of::<VirtioBlkOutHeader>() as u32,
                flags: 0, // ro
            },
            DescBuf {
                addr: data.as_mut_ptr() as u64,
                len,
                flags: 0x2, // writable, read fills buffer
            },
            DescBuf {
                addr: addr_of_mut!(status) as u64,
                len: 1,
                flags: 0x2, // write
            },
        ];

        let last = self.requestq.push_desc_raw(self.mmio_addr, &bufs, 0);
        self.requestq.wait_used(last);

        match status {
            VIRTIO_BLK_S_OK => Ok(data),
            VIRTIO_BLK_S_IOERR => Err(BlockError::DeviceError("IO Error: read operation failed")),
            _ => Err(BlockError::DeviceError(
                "Read failed: operation not supported on this device or driver",
            )),
        }
    }
    pub fn blk_read_buf(&mut self, sector: u64, data: &mut [u8]) -> Result<usize, BlockError> {
        let hdr = VirtioBlkOutHeader {
            _type: VIRTIO_BLK_T_IN,
            reserved: 0,
            sector,
        };
        let mut status = 0xFFu8;
        let addr = data.as_mut_ptr() as u64;

        let bufs = [
            DescBuf {
                addr: core::ptr::addr_of!(hdr) as u64,
                len: core::mem::size_of::<VirtioBlkOutHeader>() as u32,
                flags: 0, // ro
            },
            DescBuf {
                addr,
                len: data.len() as u32,
                flags: 0x2, // writable, read fills buffer
            },
            DescBuf {
                addr: addr_of_mut!(status) as u64,
                len: 1,
                flags: 0x2, // write
            },
        ];

        let last = self.requestq.push_desc_raw(self.mmio_addr, &bufs, 0);
        self.requestq.wait_used(last);

        // len covers all writable descriptors in the chain (data buffer + 1-byte status)
        let total_bytes_written = self.requestq.used_len_for(last) - 1;

        match status {
            VIRTIO_BLK_S_OK => Ok(total_bytes_written as usize),
            VIRTIO_BLK_S_IOERR => Err(BlockError::DeviceError("IO Error: read operation failed")),
            _ => Err(BlockError::DeviceError(
                "Read failed: operation not supported on this device or driver",
            )),
        }
    }
    pub fn blk_write(&mut self, sector: u64, offset: u64, data: &[u8]) -> Result<(), BlockError> {
        let sector_size = self.sector_size() as u64;
        let mut cur_sector = sector + offset / sector_size;
        let mut cur_offset = (offset % sector_size) as u32;
        let mut remaining = data;

        while !remaining.is_empty() {
            let chunk_len = min(remaining.len(), (sector_size as u32 - cur_offset) as usize);
            let (chunk, rest) = remaining.split_at(chunk_len);
            self.write_sector_partial(cur_sector, cur_offset, chunk)?;
            remaining = rest;
            cur_sector += 1;
            cur_offset = 0;
        }
        Ok(())
    }
    fn write_sector_partial(
        &mut self,
        sector: u64,
        offset: u32,
        data: &[u8],
    ) -> Result<(), BlockError> {
        let sector_size = self.sector_size();
        if offset as usize + data.len() > sector_size as usize {
            return Err(BlockError::OutOfBounds("Write exceeds sector size"));
        }

        let mut buf = if offset == 0 && data.len() == sector_size as usize {
            alloc::vec![0u8; sector_size as usize]
        } else {
            self.blk_read(sector, sector_size)?
        };

        buf[offset as usize..offset as usize + data.len()].copy_from_slice(data);

        let hdr = VirtioBlkOutHeader {
            _type: VIRTIO_BLK_T_OUT,
            reserved: 0,
            sector,
        };
        let mut status = 0xFFu8;

        let bufs = [
            DescBuf {
                addr: core::ptr::addr_of!(hdr) as u64,
                len: core::mem::size_of::<VirtioBlkOutHeader>() as u32,
                flags: 0, // ro
            },
            DescBuf {
                addr: buf.as_ptr() as u64,
                len: sector_size,
                flags: 0, // ro
            },
            DescBuf {
                addr: core::ptr::addr_of_mut!(status) as u64,
                len: 1,
                flags: 0x2, // write
            },
        ];

        let last = self.requestq.push_desc_raw(self.mmio_addr, &bufs, 0);
        self.requestq.wait_used(last);

        match status {
            VIRTIO_BLK_S_OK => Ok(()),
            VIRTIO_BLK_S_IOERR => Err(BlockError::DeviceError(
                "IO Error: blk write operation failed",
            )),
            _ => Err(BlockError::DeviceError(
                "Write failed: operation not supported on this device or driver",
            )),
        }
    }
    /// A driver MUST set sector to 0 for a VIRTIO_BLK_T_FLUSH request. A driver SHOULD NOT include any data in a VIRTIO_BLK_T_FLUSH request.
    pub fn blk_flush() {}
}

impl BlockDevice for VirtioBlock {
    fn sector_size(&self) -> u32 {
        self.cfg.blk_size
    }

    fn capacity(&self) -> u64 {
        self.cfg.capacity
    }
    fn read_sector(&mut self, sector: u64) -> Result<Vec<u8>, block::error::BlockError> {
        self.blk_read(sector, self.sector_size())
    }
    /// Panics if buffer len is not aligned to `self.sector_size()`
    fn read_buffer(&mut self, sector: u64, buffer: &mut [u8]) -> Result<usize, BlockError> {
        if !buffer.len().is_multiple_of(self.sector_size() as usize) {
            let aligned_len = (buffer.len() + (self.sector_size() - 1) as usize)
                & !(self.sector_size() as usize - 1);
            let mut read_buffer = alloc::vec![0u8; aligned_len];
            self.blk_read_buf(sector, &mut read_buffer)?;
            buffer.copy_from_slice(&read_buffer[..buffer.len()]);
            return Ok(buffer.len());
        }
        self.blk_read_buf(sector, buffer)
    }
    fn write_buffer(
        &mut self,
        sector: u64,
        offset: u64,
        data: &[u8],
    ) -> Result<(), block::error::BlockError> {
        self.blk_write(sector, offset, data)
    }
    fn write_sector_offset(
        &mut self,
        sector: u64,
        data: &[u8],
        offset: u64,
    ) -> Result<(), BlockError> {
        self.write_sector_partial(sector, offset as u32, data)
    }
}
