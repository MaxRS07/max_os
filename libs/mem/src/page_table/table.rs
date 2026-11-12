use core::{
    arch::asm,
    fmt::Display,
    ptr::{addr_of, null_mut},
};

use alloc::{collections::btree_map::Entry, vec::Vec};
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

/* User region of the table (4-255) */
pub const USER_VA_START: usize = 0x1_0000_0000;
pub const USER_VA_END: usize = 0x40_0000_0000;

pub const USER_ROOT_START: usize = USER_VA_START >> 30;
pub const USER_ROOT_END: usize = USER_VA_END >> 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageSize {
    /// 4 KiB page located at level 0 table
    Single = 0x1000,
    /// 2 MiB megapage located at level 1 table
    Megapage = 0x20_0000,
    /// 1 GiB gigapage on the root table
    Gigapage = 0x4000_0000,
}

impl PageSize {
    pub fn level(&self) -> usize {
        match self {
            Self::Gigapage => 2,
            Self::Megapage => 1,
            Self::Single => 0,
        }
    }
}

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
        page_size: PageSize,
    ) -> Result<(), MemoryError> {
        Self::check_range(virt_addr)?;
        Self::check_align(virt_addr, page_size)?;
        TableEntry::check_user_flags(flags)?;
        let virt_addr = aligned_down(virt_addr, LEAF_SIZE);

        let mask = 0x1FF; // 9 bit mask

        let mut table = self;
        // walk down levels
        for level in (0..=2).rev() {
            let shift = 12 + level * 9;
            let vpn = (virt_addr >> shift) & mask;

            let entry = &mut table.pages[vpn];

            if level == page_size.level() {
                if !entry.is_valid() {
                    entry.set_addr(phys_addr);
                    entry.init(flags);
                    return Ok(());
                } else {
                    return Err(MemoryError::InvalidAddress(
                        "Cannot overwrite existing mapping",
                    ));
                }
            } else if entry.is_leaf() {
                return Err(MemoryError::InvalidAddress(
                    "Cannot overwrite existing mapping",
                ));
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
        page_size: PageSize,
        flags: usize,
    ) -> Result<Vec<usize>, MemoryError> {
        Self::check_align(virt_addr, page_size)?;
        let page_count = size.div_ceil(page_size as usize);
        let mut phys_addrs = Vec::new();
        for i in 0..page_count {
            let offset = i * page_size as usize;
            let vaddr = virt_addr + offset as usize;

            // grab a fresh page and give its address to the return
            let paddr = page_allocator.alloc()? as usize;
            phys_addrs.push(paddr);

            unsafe { (paddr as *mut u8).write_bytes(0, LEAF_SIZE) }; // clear page
            self.map(page_allocator, vaddr, paddr, flags, PageSize::Single)?;
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
        Self::check_range(virt_addr)?;
        let virt_addr = aligned_down(virt_addr, LEAF_SIZE);

        let mask = 0x1FF; // 9 bit mask

        let mut table: *mut Self = self;
        let mut levels = [(null_mut(), 0); 3];
        for i in (0..=2).rev() {
            let shift = 12 + i * 9;
            let vpn = (virt_addr >> shift) & mask;

            levels[i] = (table, vpn);

            let entry = unsafe { (*table).entry_mut(vpn) };

            if !entry.is_valid() {
                return Err(MemoryError::InvalidAddress(
                    "Virtual address is already unmapped",
                ));
            }

            if entry.is_leaf() {
                let physical_address = entry.clear();

                let mut j = i;
                while j < 2 {
                    let (table, _) = levels[j];
                    let (parent, p_vpn) = levels[j + 1];

                    unsafe {
                        if !(*table).has_valid_entries() {
                            let addr = (*parent).entry_mut(p_vpn).clear();
                            page_allocator.free(addr as *mut u8)?;
                        } else {
                            break;
                        }
                    }
                    j += 1;
                }
                unsafe {
                    asm!("sfence.vma"); // reset table cache to clear unmapped memory
                }
                return Ok(physical_address);
            }
            table = unsafe { &mut *(entry.addr() as *mut Table) };
        }

        Err(MemoryError::InvalidAddress(
            "Requested virtual address is unmapped",
        ))
    }

    /// faults if the provided virtual address is outside the user range
    pub fn check_range(address: usize) -> Result<(), MemoryError> {
        if !(USER_VA_START..=USER_VA_END).contains(&address) {
            Err(MemoryError::AccessViolation(
                "Specified address is outside of user range",
            ))
        } else {
            Ok(())
        }
    }
    /// faults if an address space is not aligned to the specified page size
    pub fn check_align(address: usize, size: PageSize) -> Result<(), MemoryError> {
        let virt_addr = aligned_down(address, 0x1000);
        if virt_addr & (size as usize - 1) == 0 {
            Ok(())
        } else {
            Err(MemoryError::AlignmentError(
                "Virtual address not aligned to specified page size",
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
        let mut table = self;
        let mask = 0x1FF;

        for level in (0..=2).rev() {
            let size = 0x1000 << (level * 9);
            let shift = 12 + level * 9;
            let vpn = (virt_addr >> shift) & mask;

            let entry = table.pages[vpn];

            if !entry.is_valid() {
                return 0;
            }

            if entry.is_leaf() {
                return entry.addr() + (virt_addr & (size - 1));
            }

            if entry.is_table() {
                let table_ptr = entry.addr() as *const Table;
                table = unsafe { &*table_ptr }
            }
        }
        0
    }
    /// Gets a mutable ref of a table entry at index `idx`. Panics if `idx >= TABLE_LEN`.
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
                if !entry.is_leaf() {
                    return;
                }
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
