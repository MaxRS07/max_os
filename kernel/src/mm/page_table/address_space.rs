use alloc::vec::Vec;

use crate::mm::{
    error::MemoryError,
    heap::{page_table::table::Table, palloc::PageAllocator},
    page_table::asid::ASID,
};

/// An address space for a process. Manages the process' virtual addresses for contiguity
/// The address space must return ASID and free all tables on `Drop`
trait Addresser: Drop {
    /// Creates a new empty address space using an ASID. ASIDs must be between 0 and 511
    fn new(asid: ASID) -> Self;
    /// Unmaps this address space, freeing all of its owned regions
    fn unmap_region(&mut self);
    /// Maps virtual addresses to this space, increasing this space's capacity by [`bytes`]
    fn map_region(&mut self, region: VirtualRegion) -> Result<(), MemoryError>;
    /// Returns the address space identifier
    fn asid(&self) -> ASID;
    /// Returns the supervisor address translation and protection register (satp)
    fn satp(&self) -> usize;
}

struct VirtualRegion {
    /// the virtual address of the start of the region
    virt_addr: u64,
    /// the size of the region in bytes. The number of contiguous leaves owned by this region is [`bytes / 0x1000`] unless the page is a megatable
    size: u64,
}
/// Address space owns a root table. Address spaces are assigned per process
pub struct AddressSpace {
    /// the root table
    root_table: &'static Table,
    /// identifier of this address space
    asid: ASID,
    /// list of owned virtual regions
    regions: Vec<VirtualRegion>,
}

impl AddressSpace {
    pub fn new(root_table: *mut Table, asid: ASID, regions: Vec<VirtualRegion>) -> Self {
        Self {
            root_table,
            asid,
            regions,
        }
    }
}
