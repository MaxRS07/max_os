use core::fmt::Display;

use alloc::vec::Vec;
use sync::mutex::Mutex;

use crate::{
    align::aligned_down,
    error::MemoryError,
    page_table::{
        page_alloc::{self, PageAllocator, Pager},
        table_entry::TableEntry,
    },
};

const TABLE_LEN: usize = 512;
const LEAF_SIZE: usize = 0x1000;

/* User region of the table (256-512) */
/// 256
pub const USER_ROOT_START: usize = 0x4000_0000 >> 22;
/// 512
pub const USER_ROOT_END: usize = 0x8000_0000 >> 22;

#[repr(align(4096))]
pub struct Table {
    pub pages: [TableEntry; TABLE_LEN],
}

impl Table {
    pub const fn new() -> Self {
        Self {
            pages: [TableEntry::empty(); TABLE_LEN],
        }
    }

    /// Copies kernel mappings onto this table
    pub fn fork_kernel(&mut self, kernel: &Self) {
        self.pages[..USER_ROOT_START].copy_from_slice(&kernel.pages[..USER_ROOT_START]);
        self.pages[USER_ROOT_END..].copy_from_slice(&kernel.pages[USER_ROOT_END..]);
    }
    /// Maps a single virtual address to a physical leaf.
    ///  
    /// **Note:** forces virtual alignment to 0x1000
    pub fn map(
        &mut self,
        page_allocator: &dyn Pager,
        virt_addr: usize,
        phys_addr: usize,
        flags: usize,
    ) -> Result<(), MemoryError> {
        let virt_addr = aligned_down(virt_addr, LEAF_SIZE);

        let mask = 0x3FF; // 10 bit mask

        let mut table = self;
        // walk down levels
        for i in 0..2 {
            let shift = 12 + i * 9;
            let vpn = (virt_addr >> shift) & mask;

            let entry = &mut table.pages[vpn];

            if entry.is_leaf() {
                if !entry.is_valid() {
                    entry.set_addr(phys_addr);
                    entry.init(flags);
                    return Ok(());
                } else {
                    return Err(MemoryError::InvalidAddress(
                        "Cannot overwrite existing mapping",
                    ));
                }
            }
            // entry is table, check validity
            if !entry.is_valid() {
                let entry_table = Self::alloc_empty(page_allocator)?;
                entry.set_addr(entry_table as usize);
                entry.set_valid(true);
                table = unsafe { &mut *entry_table };
                continue;
            }

            unsafe { table = &mut *(entry.addr() as *mut Table) };
        }
        return Err(MemoryError::InvalidAddress(
            "Leaf not found for the specified address",
        ));
    }
    /// Maps virtual addresses `virt_addr` and consecutive addresses in page increments totaling `size` bytes to physical addresses, returning the physical address list in order.
    pub fn map_size(
        &mut self,
        page_allocator: &dyn Pager,
        virt_addr: usize,
        size: usize,
        flags: usize,
    ) -> Result<Vec<usize>, MemoryError> {
        let page_count = size.div_ceil(LEAF_SIZE as usize);
        let mut phys_addrs = Vec::new();
        for i in 0..page_count {
            let offset = i * LEAF_SIZE as usize;
            let vaddr = virt_addr + offset as usize;

            // grab a fresh page and give its address to the return
            let paddr = page_allocator.alloc()? as usize;
            phys_addrs.push(paddr);

            unsafe { (paddr as *mut u8).write_bytes(0, LEAF_SIZE) }; // clear page
            self.map(page_allocator, vaddr, paddr, flags)?;
        }
        Ok(phys_addrs)
    }
    /// Unmaps a virtual address from a physical address, freeing the physical page and returning [`Ok`] containing the physical address. If [`virt_addr`] is unampped, returns [`Err`]
    /// ## Warning!
    /// Megapages cannot be partially unmapped. unmapping a leaf inside a megatable probably wont do anything but it might cause corruption
    pub fn unmap(
        &mut self,
        page_allocator: &dyn Pager,
        virt_addr: usize,
    ) -> Result<usize, MemoryError> {
        let virt_addr = aligned_down(virt_addr, LEAF_SIZE);

        let vpn_root = (virt_addr >> 22) & 0x3FF;
        let vpn_leaf = (virt_addr >> 12) & 0x3FF;

        // create 4KiB megatable if both phys and vpn0 are aligned
        let child_entry = &mut self.pages[vpn_root as usize];

        if !child_entry.is_valid() {
            return Err(MemoryError::HeapCorruption(
                "Requested virtual address is unmapped",
            ));
        }

        let ptr = child_entry.addr() as *mut Table;
        unsafe {
            let child_table = &mut *ptr;
            let (leaf_was_valid, physical_address, has_valid_entries) = {
                let leaf = child_table.entry_mut(vpn_leaf);
                let leaf_was_valid = leaf.is_valid();
                let physical_address = if leaf_was_valid { leaf.clear() } else { 0 };
                (
                    leaf_was_valid,
                    physical_address,
                    child_table.has_valid_entries(),
                )
            };

            // Invalidate an empty entry table so it can be reused later.
            if !has_valid_entries {
                page_allocator.free(ptr as *mut u8)?;
            }
            if leaf_was_valid {
                return Ok(physical_address);
            }
            Err(MemoryError::AccessViolation(
                "Attemped to unmap already unmapped address",
            ))
        }
    }
    /// unmaps `size` consecutive virtual regions
    pub fn unmap_size(
        &mut self,
        page_allocator: &dyn Pager,
        virt_addr: usize,
        size: usize,
    ) -> Result<(), MemoryError> {
        let offset_size = LEAF_SIZE;
        let page_count = size.div_ceil(offset_size);
        for i in 0..page_count {
            let offset = i * offset_size;
            let vaddr = virt_addr + offset as usize;

            self.unmap(page_allocator, vaddr)?;
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
    /// Helper to allocate an table
    pub fn alloc_empty(page_allocator: &dyn Pager) -> Result<*mut Table, MemoryError> {
        let frame = page_allocator.alloc()? as *mut Table;
        unsafe {
            frame.write(Table::new());
        }
        Ok(frame)
    }
    /// deallocates the user L0 tables, freeing the physical pages
    pub fn free_user_pages(&mut self, page_allocator: &dyn Pager) {
        self.pages[USER_ROOT_START..USER_ROOT_END]
            .iter()
            .for_each(|entry| {
                let phys_ptr = entry.addr() as *mut u8;
                let _ = page_allocator.free(phys_ptr);
            });
    }
    /// Checks if any entries in the table are valid
    ///
    /// TODO: Possibly cache this
    fn has_valid_entries(&self) -> bool {
        self.pages.iter().any(|page| page.is_valid())
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}
