use core::fmt::Display;

use alloc::boxed::Box;
use fs::collections::error::FSError;
use sdt::region::{self, FDTRegion};
use table_entry::TableEntry;

use crate::mm::error::MemoryError;

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
            pages: [TableEntry(0); TABLE_LEN],
        }
    }
    pub const MEGAPAGE: usize = 1 << 8;
    /// Maps a single virtual address to a physical leaf
    pub fn map(
        &mut self,
        virt_addr: usize,
        phys_addr: usize,
        flags: usize,
    ) -> Result<(), MemoryError> {
        let align = MEGATABLE_SIZE; // 4MiB
        let flags = flags & TableEntry::FLAG_MASK;

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

        if !root_entry.is_valid() {
            let table = Self::new();
            let boxed = Box::new(table);
            let addr = Box::into_raw(boxed).addr();
            root_entry.set_addr(addr as usize);
            root_entry.set_valid(true);
            self.valid_entries += 1;
        }
        let ptr = root_entry.addr() as *mut Table;
        unsafe {
            let table = &mut *ptr;
            let leaf = table.entry_mut(vpn_leaf);

            leaf.set_addr(phys_addr);
            leaf.init(flags);
            Ok(())
        }
    }
    pub fn map_region(
        &mut self,
        virt_addr: usize,
        region: &FDTRegion,
        flags: usize,
    ) -> Result<(), MemoryError> {
        let mut offset_size = LEAF_SIZE;
        if flags & Table::MEGAPAGE != 0 {
            offset_size = MEGATABLE_SIZE;
        }
        let page_count = region.size.div_ceil(offset_size);
        for i in 0..page_count {
            let offset = i * offset_size;
            let vaddr = virt_addr + offset as usize;
            let paddr = (region.base_address + offset) as usize;

            self.map(vaddr, paddr, flags)?;
        }

        Ok(())
    }

    /// Maps an FDT region to itself
    pub fn map_region_identity(
        &mut self,
        region: &FDTRegion,
        flags: usize,
    ) -> Result<(), MemoryError> {
        self.map_region(region.base_address as usize, region, flags)
    }

    pub fn map_mmio_identity(&mut self, mmio_slots: &[FDTRegion], flags: usize) {
        for slot in mmio_slots {
            let _ = self.map_region_identity(slot, flags);
        }
    }
    /// unmaps a virtual address from a physical address, freeing the physical page
    /// ## warning
    /// megapages cannor be partially unmapped. unmapping a leaf inside a megatable probably wont do anything
    pub fn unmap(&mut self, virt_addr: usize) -> Result<usize, MemoryError> {
        let align = MEGATABLE_SIZE; // 4MiB
        let flags = flags & TableEntry::FLAG_MASK;

        let vpn_root = (virt_addr >> 22) & 0x3FF;
        let vpn_leaf = (virt_addr >> 12) & 0x3FF;

        // create 4KiB megatable if both phys and vpn0 are aligned
        let root_entry = &mut self.pages[vpn_root as usize];

        if (flags & Self::MEGAPAGE) != 0 {
            if vpn_leaf == 0 && phys_addr.is_multiple_of(align) {
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
            let leaf = table.entry_mut(vpn_leaf);

            table.valid_entries -= 1;

            if table.valid_entries == 0 {
                drop(table)
            }
            Ok(leaf.clear())
        }
    }
    /// unmaps consecutive regions at a physical address spanning the regions size. returns the first physical address of the region
    pub fn unmap_region(
        &mut self,
        virt_addr: usize,
        region: &FDTRegion,
    ) -> Result<(), MemoryError> {
        let mut offset_size = LEAF_SIZE;
        let page_count = region.size.div_ceil(offset_size);
        for i in 0..page_count {
            let offset = i * offset_size;
            let vaddr = virt_addr + offset as usize;
            let paddr = (region.base_address + offset) as usize;

            self.unmap(virt_addr)?;
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
