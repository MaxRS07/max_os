use core::fmt::Display;

use alloc::vec::Vec;
use fs::collections::error::FSError;
use mem::align::align_down;
use sdt::region::{self, FDTRegion};

use crate::mm::{error::MemoryError, heap::PAGE_ALLOCATOR, page_table::table_entry::TableEntry};

const TABLE_LEN: usize = 1024;
const LEAF_SIZE: usize = 0x1000;
const MEGATABLE_SIZE: usize = 0x400000;

#[repr(align(4096))]
pub struct Table {
    pub valid_entries: u16,
    pub pages: [TableEntry; TABLE_LEN],
}

impl Table {
    pub const fn new() -> Self {
        Self {
            valid_entries: 0,
            pages: [TableEntry::empty(); TABLE_LEN],
        }
    }
    pub const MEGAPAGE: usize = 1 << 8;

    pub fn has_valid_entries(&self) -> bool {
        self.valid_entries != 0
    }
    /// Maps a single virtual address to a physical leaf.
    ///  
    /// **Note:** forces virtual alignment to 0x1000
    pub fn map(
        &mut self,
        virt_addr: usize,
        phys_addr: usize,
        flags: usize,
    ) -> Result<(), MemoryError> {
        let align = MEGATABLE_SIZE; // 4MiB
        let virt_addr = align_down(virt_addr, LEAF_SIZE);
        let vpn_root = (virt_addr >> 22) & 0x3FF;
        let vpn_leaf = (virt_addr >> 12) & 0x3FF;

        // create 4KiB megatable if both phys and vpn0 are aligned
        let root_entry = &mut self.pages[vpn_root as usize];

        if (flags & Self::MEGAPAGE) != 0 {
            if vpn_leaf == 0 && phys_addr.is_multiple_of(align) {
                root_entry.set_addr(phys_addr);
                self.valid_entries += 1;
                root_entry.init(flags);
                return Ok(());
            } else {
                return Err(MemoryError::PageFault(
                    "Specified megatable, but address not aligned",
                ));
            }
        }

        if !root_entry.is_leaf() {
            let table = unsafe { PAGE_ALLOCATOR.get_mut().unwrap().alloc()? };
            root_entry.set_addr(table as usize);
            root_entry.set_valid(true);
            self.valid_entries += 1;
        }
        let ptr = root_entry.addr() as *mut Table;
        unsafe {
            let table = &mut *ptr;
            let leaf = table.entry_mut(vpn_leaf);
            if !leaf.is_valid() {
                leaf.set_addr(phys_addr);
                leaf.init(flags);
            } else {
                return Err(MemoryError::PageFault("Attempted to overwrite mapped leaf"));
            }
            Ok(())
        }
    }
    /// Maps virtual addresses `virt_addr` and consecutive addresses in page increments totaling `size` bytes to physical addresses, returning the physical address list in order.
    pub fn map_size(
        &mut self,
        virt_addr: usize,
        size: usize,
        flags: usize,
    ) -> Result<Vec<usize>, MemoryError> {
        let offset_size = if flags & Table::MEGAPAGE != 0 {
            MEGATABLE_SIZE
        } else {
            LEAF_SIZE
        };
        let page_count = size.div_ceil(offset_size as usize);
        let mut phys_addrs = alloc::vec![0usize; page_count as usize];
        for i in 0..page_count {
            let offset = i * offset_size as usize;
            let vaddr = virt_addr + offset as usize;

            // grab a fresh page and give its address to the return
            let paddr = unsafe {
                PAGE_ALLOCATOR
                    .get_mut()
                    .ok_or(MemoryError::NotInitialized(
                        "Failed to retrieve page allocator",
                    ))?
                    .alloc()?
            } as usize;
            phys_addrs.push(paddr);

            self.map(vaddr, paddr, flags)?;
        }
        Ok(phys_addrs)
    }
    /* Low level FDT map helpers */
    /// Maps a virtual address to its respective contiguous physical region. For non-contiguous region mapping use `map_region`
    pub fn map_region_identity(
        &mut self,
        region: &FDTRegion,
        flags: usize,
    ) -> Result<(), MemoryError> {
        let virt_addr = region.base_address;
        let offset_size = if flags & Table::MEGAPAGE != 0 {
            MEGATABLE_SIZE
        } else {
            LEAF_SIZE
        };
        let page_count = region.size.div_ceil(offset_size);
        for i in 0..page_count {
            let offset = i * offset_size;
            let vaddr = virt_addr + offset as usize;
            let paddr = (region.base_address + offset) as usize;

            self.map(vaddr, paddr, flags)?;
        }

        Ok(())
    }

    pub fn map_mmio_identity(&mut self, mmio_slots: &[FDTRegion], flags: usize) {
        for slot in mmio_slots {
            let _ = self.map_region_identity(slot, flags);
        }
    }
    /// Unmaps a virtual address from a physical address, freeing the physical page and returning the physical address. If [`virt_addr`] is an already unampped, returns 0
    /// ## Warning!
    /// Megapages cannot be partially unmapped. unmapping a leaf inside a megatable probably wont do anything but it might cause corruption
    pub fn unmap(&mut self, virt_addr: usize, is_megatable: bool) -> Result<usize, MemoryError> {
        let virt_addr = align_down(virt_addr, LEAF_SIZE);

        let vpn_root = (virt_addr >> 22) & 0x3FF;
        let vpn_leaf = (virt_addr >> 12) & 0x3FF;

        // create 4KiB megatable if both phys and vpn0 are aligned
        let root_entry = &mut self.pages[vpn_root as usize];

        if is_megatable {
            if vpn_leaf == 0 && virt_addr.is_multiple_of(MEGATABLE_SIZE) {
                self.valid_entries -= 1;
                return Ok(root_entry.clear());
            } else {
                return Err(MemoryError::PageFault(
                    "Specified megatable, but address not aligned",
                ));
            }
        }

        let ptr = root_entry.addr() as *mut Table;
        unsafe {
            let table = &mut *ptr;
            let (leaf_was_valid, physical_address, table_is_empty) = {
                let leaf = table.entry_mut(vpn_leaf);
                let leaf_was_valid = leaf.is_valid();
                let physical_address = if leaf_was_valid { leaf.clear() } else { 0 };
                (leaf_was_valid, physical_address, !table.has_valid_entries())
            };

            // Invalidate an empty entry table so it can be reused later.
            if table_is_empty {
                root_entry.clear();
            }
            if leaf_was_valid {
                self.valid_entries -= 1;
                return Ok(physical_address);
            }
            Ok(0)
        }
    }
    /// unmaps consecutive regions at a physical address spanning the regions size. returns the first physical address of the region
    pub fn unmap_region(
        &mut self,
        virt_addr: usize,
        region: &FDTRegion,
    ) -> Result<(), MemoryError> {
        let offset_size = LEAF_SIZE;
        let page_count = region.size.div_ceil(offset_size);
        for i in 0..page_count {
            let offset = i * offset_size;
            let vaddr = virt_addr + offset as usize;

            self.unmap(vaddr, false)?;
        }

        Ok(())
    }
    /// Returns the physical address asscosiated with `virt_addr`, or 0 if `virt_addr` is invalid
    pub fn translate(&self, virt_addr: usize) -> usize {
        let vpn_root = (virt_addr >> 22) & 0x3FF;
        let vpn_leaf = (virt_addr >> 12) & 0x3FF;

        let root = self.pages[vpn_root as usize];

        if !root.is_valid() {
            return 0;
        }
        if root.is_executable() || root.is_writable() || root.is_readable() {
            // root is a leaf, return its address
            return root.addr();
        }

        let leaf_table_ptr = root.addr() as *mut Table;
        unsafe {
            let leaf_table = &mut *leaf_table_ptr;
            let leaf_entry = leaf_table.entry_mut(vpn_leaf);
            if leaf_entry.is_valid() {
                return leaf_entry.addr();
            }
        }
        0
    }
    /// Gets a mutable ref of a table entry at index `idx`. Panics if `idx` is larger than 1023.
    pub fn entry_mut(&mut self, idx: usize) -> &mut TableEntry {
        &mut self.pages[idx as usize]
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}
