pub const REQUESTQ: u32 = 0;

// operation types
pub const VIRTIO_BLK_T_IN: u32 = 0;
pub const VIRTIO_BLK_T_OUT: u32 = 1;
pub const VIRTIO_BLK_T_FLUSH: u32 = 4;
pub const VIRTIO_BLK_T_DISCARD: u32 = 11;
pub const VIRTIO_BLK_T_WRITE_ZEROES: u32 = 13;

// status
pub const VIRTIO_BLK_S_OK: u8 = 0;
pub const VIRTIO_BLK_S_IOERR: u8 = 1;
pub const VIRTIO_BLK_S_UNSUPP: u8 = 2;

/// config addr, read `VirtioBlkConfig` at mmio addr offset by this value
pub const VIRTIO_BLK_CFG_OFFSET: u32 = 0x100;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct VirtioBlkGeometry {
    cylinders: u16,
    heads: u8,
    sectors: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]

struct VirtioBlkTopology {
    /// # of logical blocks per physical block (log2)
    physical_block_exp: u8,
    /// offset of first aligned logical block
    alignment_offset: u8,
    /// suggested minimum I/O size in blocks
    min_io_size: u16,
    /// optimal (suggested maximum) I/O size in blocks
    opt_io_size: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VirtioBlkConfig {
    pub capacity: u64,
    pub size_max: u32,
    pub seg_max: u32,
    pub geometry: VirtioBlkGeometry,
    pub blk_size: u32,
    pub topology: VirtioBlkTopology,
    pub writeback: u8,
    pub unused0: [u8; 3],
    pub max_discard_sectors: u32,
    pub max_discard_seg: u32,
    pub discard_sector_alignment: u32,
    pub max_write_zeroes_sectors: u32,
    pub max_write_zeroes_seg: u32,
    pub write_zeroes_may_unmap: u8,
    pub unused1: [u8; 3],
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VirtioBlkOutHeader {
    pub _type: u32,
    pub reserved: u32,
    pub sector: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Flags {
    /// only used for write zeroes command
    unmap: u32,
    reserved: u32,
}
impl Default for Flags {
    fn default() -> Self {
        Self {
            unmap: 1,
            reserved: 31,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct VirtioBlkDiscardWriteZeroes {
    sector: u64,
    num_sectors: u32,
    flags: Flags,
}

const SCSI_SENSE_BUFFERSIZE: usize = 96;
/* TODO: All fields are in guest’s native endian.  */
// struct VirtioScsiPcReq {
//     _type: u32,
//     ioprio: u32,
//     sector: u64,
//     cmd: [u8],
//     data: [u8; 512],
//     sense: [u8; SCSI_SENSE_BUFFERSIZE],
//     errors: u32,
//     data_len: u32,
//     sense_len: u32,
//     residual: u32,
//     status: u8,
// }
