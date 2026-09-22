use core::{default, ptr::read};

use alloc::vec::Vec;
use fs::meta::permission::Permissions;
use sdt::region::FDTRegion;

use crate::mm::{
    error::MemoryError,
    heap::{PAGE_ALLOCATOR, page_table::table::Table, palloc::PageAllocator},
    page_table::{asid::ASID, table::Table},
};

/// An address space for a process. Manages the process' virtual addresses for contiguity
/// The address space must return ASID and free all tables on `Drop`
trait Addresser: Drop {
    /// Creates a new empty address space using an ASID. ASIDs must be between 0 and 511
    fn new(asid: ASID) -> Self;
    /// Force-unmaps this address space, freeing all of its owned regions
    fn drop(&mut self);
    /// Unmaps `virt_addr` from this space, freeing the physical page
    fn unmap(&mut self, virt_addr: usize) -> Result<(), MemoryError>;
    /// Maps virtual addresses to this space, increasing this space's capacity by at least [`bytes`]
    fn map_size(&mut self, virt_addr: usize, size: usize, flags: usize) -> Result<(), MemoryError>;
    /// Returns the address space identifier
    fn asid(&self) -> ASID;
    /// Returns the supervisor address translation and protection register (satp)
    fn satp(&self) -> usize;
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RegionBacking {
    #[default]
    /// Anon space is fully freed and reclaimed on teardown
    Anonymous,
    /// This memory is file backed, write back diry pages
    File,
    /// This file is backed by shared space and can't be freed on tear down.
    Shared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// A virtual region held by an address space
struct VirtualRegion {
    perms: Permissions,
    /// the virtual address of the start of the region
    pub virt_addr: usize,
    /// the size of the region in pages.
    pub size: usize,
    /// whether this region is a megatable. If true, then `size` is the number of megatables in the region
    pub megatable: bool,
    /// Physical memory type
    backing: RegionBacking,
}

impl VirtualRegion {
    pub fn new(perms: Permissions, virt_addr: usize, size: usize, backing: RegionBacking) -> Self {
        Self {
            perms,
            virt_addr,
            size,
            backing,
            // dont want to deal with this right now
            megatable: false,
        }
    }
    pub fn contains_virtual_address(&self, virt_addr: usize) -> bool {
        self.virt_addr <= virt_addr && self.virt_addr + self.size > virt_addr
    }
}

/// Address space of a process. Groups memory by process and holds mapped regions that are accessed as contiguous chuncks by processes
pub struct AddressSpace {
    /// the root table
    root_table: &'static mut Table,
    /// identifier of this address space
    asid: ASID,
    /// list of owned virtual regions
    regions: Vec<VirtualRegion>,
}

impl AddressSpace {
    pub fn new(root_table: &'static mut Table, asid: ASID, regions: Vec<VirtualRegion>) -> Self {
        Self {
            root_table,
            asid,
            regions,
        }
    }
}

impl Addresser for AddressSpace {
    fn new(asid: ASID) -> Self {
        todo!()
    }

    fn drop(&mut self) {
        for virt_reg in self.regions {
            self.root_table.unmap(virt_reg.virt_addr);
        }
    }

    fn unmap(&mut self, virt_addr: usize) -> Result<(), MemoryError> {
        virt_addr = virt_addr.next_multiple_of(0x1000);
        for index in 0..self.regions.len() {
            let region = self.regions[index];
            // easy
            if region.virt_addr == virt_addr {
                self.root_table.unmap(virt_addr);
                self.regions.remove(index);
                return Ok(());
            }
            // need to split region
            if region.contains_virtual_address(virt_addr) {
                let l = VirtualRegion {
                    size: region.size - 1,
                    ..region
                };
                let r = VirtualRegion {
                    virt_addr: virt_addr + 0x1000,
                    size: region.size - 1,
                    ..region
                };
                self.regions.remove(index);
                self.regions.insert(index, r);
                self.regions.insert(index, l);
            }
        }
        Err(MemoryError::AccessViolation(
            "Attemped to free unmapped region from address space",
        ))
    }

    fn map_size(
        &mut self,
        virt_addr: usize,
        size: usize,
        flags: usize,
        perms: Permissions,
    ) -> Result<(), MemoryError> {
        let phys_addrs = self.root_table.map_size(virt_addr, size, flags)?;
        let mut peekable_addrs = phys_addrs.iter().peekable();
        let mut cur_size = 0x1000;
        for addr in peekable_addrs {
            /// save space by grouping contiguous virt regions. This will be a massive headache later on partial frees
            if let Some(next) = peekable_addrs.peek()
                && (addr + 0x1000).eq(*next)
            {
                cur_size += 0x1000;
                continue;
            }
            let virt_region =
                VirtualRegion::new(perms, virt_addr, cur_size, RegionBacking::Anonymous);
            self.regions.push(virt_region);
        }
        Ok(())
    }

    fn asid(&self) -> ASID {
        todo!()
    }

    fn satp(&self) -> usize {
        todo!()
    }
}
