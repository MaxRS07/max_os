use core::fmt::{Debug, Display};

use crate::meta::inode::SECTOR_SIZE;

// Sector address readability and utilities
pub type SectorAddr = u64;

#[derive(Clone, Hash, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

/// Wrapper for a sector address. Uses upper 63 bits for the sector and reserves the lowest bit for the subsector offset
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FSHeaderSector {
    packing: SectorPacking,
    address: u64,
}

impl FSHeaderSector {
    /// constructs
    pub const fn new(address: u64) -> Self {
        let packing = SectorPacking::new(SECTOR_SIZE);
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

impl Display for FSHeaderSector {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let sector = self.sector();
        let offset = self.offset();
        let pack = self.packing;
        f.write_fmt(format_args!(
            "FSHeaderSector(packing:{pack:?}, sector:{sector}, offset:{offset})"
        ))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Debug, Eq, PartialOrd, Ord)]
pub struct FSInodeSector {
    packing: SectorPacking,
    address: u64,
}

impl FSInodeSector {
    /// constructs
    pub const fn new(address: u64) -> Self {
        // the offset half of the address is a *byte* offset inside the sector, so the
        // packing has to reserve enough low bits for a full sector, not for the number
        // of inodes per sector.
        let packing = SectorPacking::new(SECTOR_SIZE);
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
impl Display for FSInodeSector {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let sector = self.sector();
        let offset = self.offset();
        f.write_fmt(format_args!(
            "FSInodeSector(sector:{sector}, offset:{offset})"
        ))
    }
}
