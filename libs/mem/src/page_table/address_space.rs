use core::{
    default,
    ptr::{null_mut, read},
};

use alloc::vec::{self, Vec};

use crate::{
    align::aligned_down,
    error::MemoryError,
    page_table::{asid::ASID, page_alloc::Pager, permissions::PagePermissions, table::Table},
};

/// An address space for a process. Manages the process' virtual addresses for contiguity
/// The address space must return ASID and free all tables on `Drop`
trait Addresser {
    /// Creates a new empty address space using an ASID. ASIDs must be between 0 and 511. This method will `Err` if the page
    /// allocator runs out of pages or fails to allocate
    // fn new(asid: ASID) -> Result<Self, MemoryError>;
    /// Force-unmaps this address space, freeing all of its owned regions
    fn drop(&mut self) -> Result<(), MemoryError>;
    /// Unmaps `virt_addr` from this space, freeing the physical page
    fn unmap(&mut self, virt_addr: usize) -> Result<(), MemoryError>;
    /// Maps virtual addresses to this space, increasing this space's capacity by at least [`bytes`]
    fn map_size(
        &mut self,
        virt_addr: usize,
        size: usize,
        flags: usize,
        perms: PagePermissions,
    ) -> Result<(), MemoryError>;
    /// Translates a virtual address within this space to its corresponding physical address
    fn translate(&self, virt_addr: usize) -> Result<usize, MemoryError>;
    /// Checks if this addresser contains a virtual address
    fn contains(&self, virt_addr: usize) -> bool;
    /// Returns the address space identifier
    fn asid(&self) -> ASID;
    /// Returns the supervisor address translation and protection register (satp)
    fn satp(&self) -> usize;
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
enum RegionBacking {
    #[default]
    /// Anon space is fully freed and reclaimed on teardown
    Anonymous,
    /// This memory is file backed, write back diry pages
    File,
    /// This file is backed by shared space and can't be freed on tear down.
    Shared,
}

#[derive(Clone, Copy)]
/// A virtual region held by an address space
pub struct VirtualRegion {
    perms: PagePermissions,
    /// the virtual address of the start of the region
    pub virt_addr: usize,
    /// the physical address of the first page in the region. If `size > 1`, then the following physical addresses will be offset by 0x1000 up to `size`
    pub phys_addr: usize,
    /// the size of the region in pages.
    pub size: usize,
    /// whether this region is a megatable. If true, then `size` is the number of megatables in the region
    pub megatable: bool,
    /// Physical memory type
    backing: RegionBacking,
}

impl VirtualRegion {
    pub fn new(
        perms: PagePermissions,
        virt_addr: usize,
        phys_addr: usize,
        size: usize,
        backing: RegionBacking,
    ) -> Self {
        Self {
            perms,
            virt_addr,
            phys_addr,
            size,
            backing,
            // dont want to deal with this right now
            megatable: false,
        }
    }
    pub fn contains_virtual_address(&self, virt_addr: usize) -> bool {
        self.virt_addr <= virt_addr && self.virt_addr + self.size > virt_addr
    }
    /// Returns virtual address `virt_addr` to its corresponding physical address. Resturns `MemoryError` if the virtual address is not contained within the region
    pub fn translate(&self, virt_addr: usize) -> Result<usize, MemoryError> {
        if !self.contains_virtual_address(virt_addr) {
            return Err(MemoryError::AccessViolation(
                "Virtual address not contained in address space",
            ));
        }
        Ok(virt_addr - self.virt_addr + self.phys_addr)
    }
}

/// Address space of a process. Groups memory by process and holds mapped regions that are accessed as contiguous chuncks by processes
#[derive(Clone)]
pub struct AddressSpace {
    page_allocator: &'static dyn Pager,
    /// Pointer to the in-memory root table, this lives on the root page
    root_table: *mut Table,
    /// identifier of this address space
    asid: ASID,
    /// list of owned virtual regions
    regions: Vec<VirtualRegion>,
}

impl AddressSpace {
    // pub fn kernel() -> Self {
    //     Self::new(, 0, regions);
    // }
    pub fn new(
        page_allocator: &'static dyn Pager,
        root_table: *mut Table,
        asid: ASID,
        regions: Vec<VirtualRegion>,
    ) -> Self {
        Self {
            page_allocator,
            root_table,
            asid,
            regions,
        }
    }
    fn from_asid(page_allocator: &'static dyn Pager, asid: ASID) -> Result<Self, MemoryError> {
        // create the root table. This table must be zerod
        let root_ptr = Table::alloc_empty(page_allocator)?;
        Ok(Self::new(page_allocator, root_ptr, asid, Vec::new()))
    }
}

impl Addresser for AddressSpace {
    fn drop(&mut self) -> Result<(), MemoryError> {
        while let Some(virt_addr) = self.regions.first().map(|region| region.virt_addr) {
            self.unmap(virt_addr)?;
        }
        let root_table_ptr = self.root_table as *mut u8;
        self.page_allocator.free(root_table_ptr)
    }

    fn unmap(&mut self, virt_addr: usize) -> Result<(), MemoryError> {
        let virt_addr = aligned_down(virt_addr, 0x1000);
        for index in 0..self.regions.len() {
            let region = self.regions[index];
            // need to split region
            if region.contains_virtual_address(virt_addr) {
                self.regions.remove(index);
                if region.size <= 1 {
                    return Ok(());
                }
                let l = VirtualRegion {
                    size: region.size - 1,
                    ..region
                };
                let next_virt = virt_addr + 0x1000;
                let r = VirtualRegion {
                    virt_addr: next_virt,
                    phys_addr: region.translate(next_virt)?,
                    size: region.size - 1,
                    ..region
                };
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
        perms: PagePermissions,
    ) -> Result<(), MemoryError> {
        let phys_addrs = unsafe { &mut *self.root_table }.map_size(
            self.page_allocator,
            virt_addr,
            size,
            flags,
        )?;
        let mut peekable_addrs = phys_addrs.iter().peekable();
        let mut cur_size = 0x1000;
        for addr in peekable_addrs.next() {
            // save space by grouping contiguous virt regions. This will be a massive headache later on partial frees
            if let Some(next) = peekable_addrs.peek()
                && (addr + 0x1000).eq(*next)
            {
                cur_size += 0x1000;
                continue;
            }
            let virt_region =
                VirtualRegion::new(perms, virt_addr, *addr, cur_size, RegionBacking::Anonymous);
            self.regions.push(virt_region);
        }
        Ok(())
    }

    fn asid(&self) -> ASID {
        self.asid
    }

    fn satp(&self) -> usize {
        0
    }

    fn translate(&self, virt_addr: usize) -> Result<usize, MemoryError> {
        for region in self.regions.iter() {
            if let Ok(paddr) = region.translate(virt_addr) {
                return Ok(paddr);
            }
        }
        Err(MemoryError::AccessViolation(
            "virtual address not contained in address space",
        ))
    }

    fn contains(&self, virt_addr: usize) -> bool {
        self.regions
            .iter()
            .any(|region| region.contains_virtual_address(virt_addr))
    }
}
