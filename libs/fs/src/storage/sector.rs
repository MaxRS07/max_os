use crate::meta::{
    header::FSHeader,
    inode::{FSInode, SECTOR_SIZE},
};

// Sector address readability and utilities
pub type SectorAddr = u64;

#[derive(Clone, Hash, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SectorPacking {
    shift: u64,
    mask: u64,
}

impl SectorPacking {
    pub const fn new(items_per_sector: u64) -> Self {
        assert!(items_per_sector.is_power_of_two());
        let shift = items_per_sector.ilog2() as u64;
        let mask = (1 << shift) - 1;
        Self { shift, mask }
    }

    pub const fn pack(&self, sector: u64, offset: u64) -> u64 {
        (sector << self.shift) | (offset & self.mask)
    }

    pub const fn sector(&self, addr: u64) -> u64 {
        addr >> self.shift
    }

    pub const fn offset(&self, addr: u64) -> u64 {
        addr & self.mask
    }
}

const HEADER_OFFSET_MASK: u64 = SECTOR_SIZE / size_of::<FSInode>() as u64;
const HEADER_OFFSET_SHIFT: u64 = HEADER_OFFSET_MASK.ilog2() as u64;
const _: () = assert!(u64::is_power_of_two(HEADER_OFFSET_MASK));

/// Wrapper for a sector address. Uses upper 63 bits for the sector and reserves the lowest bit for the subsector offset
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FSHeaderSector {
    packing: SectorPacking,
    address: u64,
}

impl FSHeaderSector {
    /// constructs
    pub const fn new(address: u64) -> Self {
        let packing = SectorPacking::new(2);
        Self { packing, address }
    }
    pub fn from_sector_offset(sector: u64, offset: u64) -> Self {
        let mut new = Self::new(0);
        new.set_sector(sector);
        new.set_offset(offset);
        new
    }
    pub fn get(&self) -> u64 {
        self.address
    }
    /// Sector that the address points to
    pub fn sector(&self) -> u64 {
        self.packing.sector(self.address)
    }
    /// The offset of the address within the sector in bytes
    pub fn offset(&self) -> u64 {
        self.packing.offset(self.address)
    }
    pub fn set_sector(&mut self, sector: u64) {
        let offset = self.offset();
        self.address = self.packing.pack(sector, offset)
    }
    pub fn set_offset(&mut self, offset: u64) {
        let sector = self.sector();
        self.address = self.packing.pack(sector, offset)
    }
    pub fn is_null(&self) -> bool {
        self.address == 0
    }
}

impl From<FSHeaderSector> for u64 {
    fn from(value: FSHeaderSector) -> Self {
        value.address
    }
}

#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FSInodeSector {
    packing: SectorPacking,
    address: u64,
}

impl FSInodeSector {
    /// constructs
    pub const fn new(address: u64) -> Self {
        let packing = SectorPacking::new(4);
        Self { packing, address }
    }
    pub fn from_sector_offset(sector: u64, offset: u64) -> Self {
        let mut new = Self::new(0);
        new.set_sector(sector);
        new.set_offset(offset);
        new
    }
    pub fn get(&self) -> u64 {
        self.address
    }
    /// Sector that the address points to
    pub fn sector(&self) -> u64 {
        self.packing.sector(self.address)
    }
    /// The offset of the address within the sector in bytes
    pub fn offset(&self) -> u64 {
        self.packing.offset(self.address)
    }
    pub fn set_sector(&mut self, sector: u64) {
        let offset = self.offset();
        self.address = self.packing.pack(sector, offset)
    }
    pub fn set_offset(&mut self, offset: u64) {
        let sector = self.sector();
        self.address = self.packing.pack(sector, offset)
    }
    pub fn is_null(&self) -> bool {
        self.address == 0
    }
}

impl From<FSInodeSector> for u64 {
    fn from(value: FSInodeSector) -> Self {
        value.address
    }
}
