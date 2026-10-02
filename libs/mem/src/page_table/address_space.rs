use core::{
    default,
    fmt::Debug,
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
pub trait Addresser {
    /// Creates a new empty address space using an ASID. ASIDs must be between 0 and 511. This method will `Err` if the page
    /// allocator runs out of pages or fails to allocate
    fn new(page_allocator: &'static dyn Pager, root_table: *mut Table, asid: ASID) -> Self;
    /// Force-unmaps this address space, freeing all of its owned regions
    fn destroy(&mut self) -> Result<(), MemoryError>;
    /// Unmaps `virt_addr` from this space, freeing the physical page
    fn unmap_pages(&mut self, virt_addr: usize, size: usize) -> Result<(), MemoryError>;
    /// Maps virtual addresses to this space, increasing this space's capacity by at least [`bytes`]
    fn map_pages(
        &mut self,
        virt_addr: usize,
        num_pages: usize,
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

#[derive(Clone, Copy, Debug)]
/// A virtual region held by an address space
pub struct VirtualRegion {
    perms: PagePermissions,
    /// the virtual address of the start of the region
    pub virt_addr: usize,
    /// the physical address of the first page in the region. If `size > 1`, then the following physical addresses will be offset by 0x1000 up to `size`
    pub phys_addr: usize,
    /// the size of the region in pages.
    pub pages: usize,
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
        pages: usize,
        backing: RegionBacking,
    ) -> Self {
        Self {
            perms,
            virt_addr,
            phys_addr,
            pages,
            backing,
            // dont want to deal with this right now
            megatable: false,
        }
    }
    /// The size of this region in bytes
    pub fn size(&self) -> usize {
        self.pages * 0x1000
    }
    /// The last address in this region (inclusive), not to be confused with the last virtual address mapped to this region.
    ///
    /// *This value is equal to the last virtual address + `0x0FFF`.*
    pub fn end(&self) -> usize {
        self.virt_addr + self.size() - 1
    }
    pub fn contains_virtual_address(&self, virt_addr: usize) -> bool {
        self.virt_addr <= virt_addr && virt_addr <= self.end()
    }
    /// Merges this region with `next` if the merge is possible, mutating self's page count
    pub fn merge_next(&mut self, next: Self) -> bool {
        if self.megatable == next.megatable
            && self.backing == next.backing
            && self.perms == next.perms
            && self.phys_addr + self.size() + 1 == next.phys_addr
            && self.virt_addr + self.size() + 1 == next.virt_addr
        {
            self.pages += next.pages;
            return true;
        }
        false
    }
    /// Returns virtual address `virt_addr` to its corresponding physical address. Resturns `MemoryError` if the virtual address is not contained within the region
    pub fn translate(&self, virt_addr: usize) -> Result<usize, MemoryError> {
        if !self.contains_virtual_address(virt_addr) {
            return Err(MemoryError::AccessViolation(
                "Virtual address not contained in region",
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
    fn insert_region(&mut self, region: VirtualRegion) -> Result<(), MemoryError> {
        match self
            .regions
            .binary_search_by(|reg| reg.virt_addr.cmp(&region.virt_addr))
        {
            // this should probably never happen. maybe merge lengths here but idk
            Ok(_) => Err(MemoryError::AccessViolation(
                "Failed to insert region, already mapped",
            )),
            Err(idx) => {
                self.regions.insert(idx, region);
                Ok(())
            }
        }
    }
    /// Merges owned regions with identical configurations
    fn merge_regions(&mut self) {
        // lookahead + merge
        for i in 0..self.regions.len() - 1 {
            let region = self.regions[i];
            let next = self.regions[i + 1];

            if region.merge_next(next_reg) {
                self.regions.remove(i + 1);
            }
        }
    }
}

impl Addresser for AddressSpace {
    fn new(page_allocator: &'static dyn Pager, root_table: *mut Table, asid: ASID) -> Self {
        AddressSpace::new(page_allocator, root_table, asid, Vec::new())
    }
    fn destroy(&mut self) -> Result<(), MemoryError> {
        for region in self.regions.clone() {
            let _ = self.unmap_pages(region.virt_addr, region.pages);
        }
        let root_table_ptr = self.root_table as *mut u8;
        self.page_allocator.free(root_table_ptr)
    }
    /// Unmaps `num_pages` consecutive virtual addresses starting at `virt_addr` and incrementing by 0x1000. To unmap a single address, use size 1.
    fn unmap_pages(&mut self, virt_addr: usize, num_pages: usize) -> Result<(), MemoryError> {
        if num_pages == 0 {
            return Ok(());
        }

        let virt_addr = aligned_down(virt_addr, 0x1000);
        let end = virt_addr + (num_pages - 1) * 0x1000;

        if let Some(index) = self
            .regions
            .iter()
            .position(|r| r.contains_virtual_address(virt_addr) && r.contains_virtual_address(end))
        {
            let region = self.regions.remove(index);

            let left_pages = (virt_addr - region.virt_addr) / 0x1000;
            let right_pages = region.pages.saturating_sub(left_pages + num_pages);

            if left_pages > 0 {
                self.insert_region(VirtualRegion {
                    pages: left_pages,
                    ..region
                });
            }

            if right_pages > 0 {
                self.insert_region(VirtualRegion {
                    virt_addr: end + 0x1000,
                    phys_addr: region.phys_addr + (left_pages + num_pages) * 0x1000,
                    pages: right_pages,
                    ..region
                });
            }

            return Ok(());
        }

        Err(MemoryError::AccessViolation(
            "Attempted to unmap or access regions mapped to another process",
        ))
    }

    fn map_pages(
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
        let mut cur_pages = 1;
        while let Some(addr) = peekable_addrs.next() {
            // save space by grouping contiguous virt regions. This will be a massive headache later on partial frees
            if let Some(next) = peekable_addrs.peek()
                && (addr + 0x1000).eq(*next)
            {
                cur_pages += 1;
                continue;
            }
            let virt_region =
                VirtualRegion::new(perms, virt_addr, *addr, cur_pages, RegionBacking::Anonymous);
            self.regions.push(virt_region);
        }
        Ok(())
    }

    fn asid(&self) -> ASID {
        self.asid
    }
    fn satp(&self) -> usize {
        const MODE_SV32: usize = 1 << 31; // select first bit
        let ppn = (self.root_table as usize) >> 12;
        let asid = self.asid.value() as usize;

        MODE_SV32 | asid << 22 | ppn & 0x3FFFFF
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
            .any(|region: &VirtualRegion| region.contains_virtual_address(virt_addr))
    }
}

impl Debug for AddressSpace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AddressSpace")
            .field("root_table", &self.root_table)
            .field("asid", &self.asid)
            .field("regions", &self.regions)
            .finish()
    }
}
