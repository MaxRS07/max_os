use crate::storage::sector::FSHeaderSector;

pub const SUPERBLOCK_MAGIC: u32 = 0xB00B1E5;

/// On-disk struct representing a volume
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Superblock {
    magic: u32,
    pub version: u32,
    pub root: FSHeaderSector,
    pub capacity: u64,
    pub inodes: u32,
}

impl Superblock {
    pub fn new(version: u32, root: FSHeaderSector, capacity: u64, inodes: u32) -> Self {
        Self {
            magic: SUPERBLOCK_MAGIC,
            version,
            root,
            capacity,
            inodes,
        }
    }
    pub fn verify(&self) -> bool {
        self.magic == SUPERBLOCK_MAGIC
    }
}
