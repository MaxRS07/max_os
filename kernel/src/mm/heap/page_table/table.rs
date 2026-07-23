use alloc::boxed::Box;
use sdt::region::{self, FDTRegion};

use crate::{arch::riscv::csr::Csr::FFLAGS, console::writer::println, println};

const TABLE_LEN: usize = 1024;
const PAGE_SIZE: usize = 0x1000;

#[repr(align(4096))]
pub struct Table {
    /// stores
    pub pages: [TableEntry; TABLE_LEN],
}

impl Table {
    pub const fn new() -> Self {
        Self {
            pages: [TableEntry(0); TABLE_LEN],
        }
    }
    pub const MEGAPAGE: u32 = 1 << 8;
    pub fn map(&mut self, virt_addr: u32, phys_addr: u32, flags: u32) -> Result<(), &'static str> {
        let align = 0x400000; // 4MiB
        let flags = flags & TableEntry::FLAG_MASK;

        let vpn_root = (virt_addr >> 22) & 0x3FF;
        let vpn_leaf = (virt_addr >> 12) & 0x3FF;

        // create 4KiB megatable if both phys and vpn0 are aligned
        let root_entry = &mut self.pages[vpn_root as usize];

        if (flags & Self::MEGAPAGE) != 0 {
            if vpn_leaf == 0 && phys_addr.is_multiple_of(align) {
                root_entry.set_addr(phys_addr);
                root_entry.init(flags);
                return Ok(());
            } else {
                return Err("Specified megatable, but address not aligned");
            }
        }

        if !root_entry.is_valid() {
            let table = Self::new();
            let boxed = Box::new(table);
            let addr = Box::into_raw(boxed).addr();
            root_entry.set_addr(addr as u32);
            root_entry.set_valid(true);
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
        virt_addr: u32,
        region: &FDTRegion,
        flags: u32,
    ) -> Result<(), &'static str> {
        let page_count = region.size.div_ceil(PAGE_SIZE);

        for i in 0..page_count {
            let offset = i * PAGE_SIZE;
            let vaddr = virt_addr + offset as u32;
            let paddr = (region.base_address + offset) as u32;

            self.map(vaddr, paddr, flags)?;
        }

        Ok(())
    }

    /// Maps an FDT region to itself
    pub fn map_region_identity(
        &mut self,
        region: &FDTRegion,
        flags: u32,
    ) -> Result<(), &'static str> {
        self.map_region(region.base_address as u32, region, flags)
    }

    pub fn map_mmio_identity(&mut self, mmio_slots: &[FDTRegion], flags: u32) {
        for slot in mmio_slots {
            let _ = self.map_region_identity(slot, flags);
        }
    }

    pub fn map_mmio(pa: usize, size: usize, flags: u32) {}
    /// Returns the physical address asscosiated with `virt_addr`, or 0 if `virt_addr` is invalid
    pub fn translate(&self, virt_addr: u32) -> u32 {
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
    pub fn entry_mut(&mut self, idx: u32) -> &mut TableEntry {
        &mut self.pages[idx as usize]
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct TableEntry(pub u32);

impl TableEntry {
    // Sv32 Page Table Entry Flag Bits (Bits 0-7)
    pub const VALID: u32 = 1 << 0;
    pub const READ: u32 = 1 << 1;
    pub const WRITE: u32 = 1 << 2;
    pub const EXECUTE: u32 = 1 << 3;
    pub const USER: u32 = 1 << 4;
    pub const GLOBAL: u32 = 1 << 5;
    pub const ACCESSED: u32 = 1 << 6;
    pub const DIRTY: u32 = 1 << 7;

    /// Flag mask (bits 0-9 reserved)
    const FLAG_MASK: u32 = 0x3FF;
    /// PNN 1 mask (20-31)
    const PNN1_MASK: u32 = 0xFFF0_0000;
    /// PNN 0 mask (10-19)
    const PNN0_MASK: u32 = 0x000F_FC00;

    // Raw Flag Getters and Setters
    pub fn get_flags(&self) -> u32 {
        self.0 & Self::FLAG_MASK
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.0 = (self.0 & !Self::FLAG_MASK) | (flags & Self::FLAG_MASK);
    }
    /// initializes an entry. Sets flags to `flags` and sets `accessed`, `dirty`, and `valid` to `true`
    pub fn init(&mut self, flags: u32) {
        self.set_flags(flags);
        self.set_ready();
    }

    pub fn set_ready(&mut self) {
        self.set_valid(true);
        self.set_accessed(true);
        self.set_dirty(true);
    }

    pub fn set_addr(&mut self, phys_addr: u32) {
        let ppn = phys_addr >> 12;
        self.set_ppn0(ppn & 0x3FF); // low 10 bits
        self.set_ppn1((ppn >> 10) & 0xFFF); // high 12 bits
    }

    pub fn addr(&self) -> u32 {
        let ppn = self.ppn0() | (self.ppn1() << 10);
        ppn << 12
    }

    pub fn set_ppn0(&mut self, ppn0: u32) {
        self.0 &= !Self::PNN0_MASK;
        self.0 |= (ppn0 << 10) & Self::PNN0_MASK;
    }
    pub fn ppn0(&self) -> u32 {
        (self.0 & Self::PNN0_MASK) >> 10
    }

    pub fn set_ppn1(&mut self, ppn1: u32) {
        self.0 &= !Self::PNN1_MASK;
        self.0 |= (ppn1 << 20) & Self::PNN1_MASK;
    }
    pub fn ppn1(&self) -> u32 {
        (self.0 & Self::PNN1_MASK) >> 20
    }

    // Dirty
    pub fn is_dirty(&self) -> bool {
        self.0 & Self::DIRTY != 0
    }

    pub fn set_dirty(&mut self, v: bool) {
        if v {
            self.0 |= Self::DIRTY;
        } else {
            self.0 &= !Self::DIRTY;
        }
    }

    // Accessed
    pub fn is_accessed(&self) -> bool {
        self.0 & Self::ACCESSED != 0
    }

    pub fn set_accessed(&mut self, v: bool) {
        if v {
            self.0 |= Self::ACCESSED;
        } else {
            self.0 &= !Self::ACCESSED;
        }
    }

    // User Mode
    pub fn is_user_mode(&self) -> bool {
        self.0 & Self::USER != 0
    }

    pub fn set_user_mode(&mut self, v: bool) {
        if v {
            self.0 |= Self::USER;
        } else {
            self.0 &= !Self::USER;
        }
    }

    // Executable
    pub fn is_executable(&self) -> bool {
        self.0 & Self::EXECUTE != 0
    }

    pub fn set_executable(&mut self, v: bool) {
        if v {
            self.0 |= Self::EXECUTE;
        } else {
            self.0 &= !Self::EXECUTE;
        }
    }

    // Writable
    pub fn is_writable(&self) -> bool {
        self.0 & Self::WRITE != 0
    }

    pub fn set_writable(&mut self, v: bool) {
        if v {
            self.0 |= Self::WRITE;
        } else {
            self.0 &= !Self::WRITE;
        }
    }

    // Readable
    pub fn is_readable(&self) -> bool {
        self.0 & Self::READ != 0
    }

    pub fn set_readable(&mut self, v: bool) {
        if v {
            self.0 |= Self::READ;
        } else {
            self.0 &= !Self::READ;
        }
    }

    // Valid
    pub fn is_valid(&self) -> bool {
        self.0 & Self::VALID != 0
    }

    pub fn set_valid(&mut self, v: bool) {
        if v {
            self.0 |= Self::VALID;
        } else {
            self.0 &= !Self::VALID;
        }
    }

    // valid entry with R, W, or X flags set is a megapage.
    pub fn is_leaf(&self) -> bool {
        self.is_valid() && (self.is_readable() || self.is_writable() || self.is_executable())
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    // static
    pub fn ppn0_from_u32(virt_addr: u32) -> u32 {
        let mut te = Self(0);
        te.set_addr(virt_addr);
        te.ppn0()
    }
    pub fn ppn1_from_u32(virt_addr: u32) -> u32 {
        let mut te = Self(0);
        te.set_addr(virt_addr);
        te.ppn1()
    }
}
