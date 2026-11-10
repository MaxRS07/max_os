use core::fmt::Display;

use crate::error::MemoryError;

#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct TableEntry(usize);

impl TableEntry {
    // Sv39 Page Table Entry Flag Bits (Bits 0-7)
    pub const VALID: usize = 1 << 0;
    pub const READ: usize = 1 << 1;
    pub const WRITE: usize = 1 << 2;
    pub const EXECUTE: usize = 1 << 3;
    pub const USER: usize = 1 << 4;
    pub const GLOBAL: usize = 1 << 5;
    pub const ACCESSED: usize = 1 << 6;
    pub const DIRTY: usize = 1 << 7;

    /// Reserved bits (63-54) 10 bits
    const RESERVED: usize = 0xFFC0_0000_0000_0000;

    /// PPN 2 mask (bits 53-28) 26 bits
    const PPN2_MASK: usize = 0x003F_FFFF_F000_0000;

    /// PPN 1 mask (bits 27-19) 9 bits
    const PPN1_MASK: usize = 0x0000_0000_0FF8_0000;

    /// PPN 0 mask (bits 18-10) 9 bits
    const PPN0_MASK: usize = 0x0000_0000_0007_FC00;

    /// RSW reserved for software (bits 9-8) 2 bits
    const RSW_MASK: usize = 0x0000_0000_0000_0300;

    /// Flag mask (bits 7-0) 8 bits
    const FLAG_MASK: usize = 0x0000_0000_0000_00FF;

    pub const fn empty() -> Self {
        Self(0)
    }
    // Raw Flag Getters and Setters
    pub fn get_flags(&self) -> usize {
        self.0 & Self::FLAG_MASK
    }
    /// Errs if conflicting flags are detected, returns masked flags if ok
    pub fn check_flags(flags: usize) -> Result<usize, MemoryError> {
        if flags & Self::USER != 0 && flags & Self::WRITE != 0 && flags & Self::EXECUTE != 0 {
            return Err(MemoryError::AccessViolation(
                "User facing entries must enforce W^X access",
            ));
        }
        Ok(flags & Self::FLAG_MASK)
    }
    pub fn set_flags(&mut self, flags: usize) {
        self.0 = (self.0 & !Self::FLAG_MASK) | (flags & Self::FLAG_MASK);
    }
    /// initializes an entry. Sets flags to `flags` and sets `accessed`, `dirty`, and `valid` to `true`
    pub fn init(&mut self, flags: usize) {
        self.set_flags(flags);
        self.set_ready();
    }

    pub fn set_ready(&mut self) {
        self.set_valid(true);
        self.set_accessed(true);
        self.set_dirty(true);
    }

    pub fn set_addr(&mut self, phys_addr: usize) {
        let ppn = phys_addr >> 12;
        self.set_ppn0(ppn & 0x1FF); // 9 bits
        self.set_ppn1((ppn >> 9) & 0x1FF); // 9 bits
        self.set_ppn2((ppn >> 18) & 0x03FF_FFFF);
    }

    pub fn addr(&self) -> usize {
        self.ppn2() << 30 | self.ppn1() << 21 | self.ppn0() << 12
    }

    pub fn set_ppn0(&mut self, ppn0: usize) {
        self.0 &= !Self::PPN0_MASK;
        self.0 |= (ppn0 << 10) & Self::PPN0_MASK;
    }

    pub fn ppn0(&self) -> usize {
        (self.0 & Self::PPN0_MASK) >> 10
    }

    pub fn set_ppn1(&mut self, ppn1: usize) {
        self.0 &= !Self::PPN1_MASK;
        self.0 |= (ppn1 << 19) & Self::PPN1_MASK;
    }
    pub fn ppn1(&self) -> usize {
        (self.0 & Self::PPN1_MASK) >> 19
    }

    pub fn set_ppn2(&mut self, ppn2: usize) {
        self.0 &= !Self::PPN2_MASK;
        self.0 |= (ppn2 << 28) & Self::PPN2_MASK;
    }
    pub fn ppn2(&self) -> usize {
        (self.0 & Self::PPN2_MASK) >> 28
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

    // True if the table entry is mapped. If the table entry is not mapped, mapping operations suck as translations will cause page faults.
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

    pub fn is_table(&self) -> bool {
        !self.is_leaf()
    }

    // valid entry with R, W, or X flags set is a megapage.
    pub fn is_leaf(&self) -> bool {
        self.is_valid() && (self.is_readable() || self.is_writable() || self.is_executable())
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    /// resets the table entry, returning the address before the clear
    pub fn clear(&mut self) -> usize {
        let phys = self.addr();
        self.0 = 0;
        self.set_flags(0);
        phys
    }

    // static
    pub fn ppn0_from_usize(virt_addr: usize) -> usize {
        let mut te = Self(0);
        te.set_addr(virt_addr);
        te.ppn0()
    }
    pub fn ppn1_from_usize(virt_addr: usize) -> usize {
        let mut te = Self(0);
        te.set_addr(virt_addr);
        te.ppn1()
    }
}

impl Display for TableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "addr: 0x{:x}, ppn1: {}, ppn0: {}, R: {}, W: {}, X: {}, V: {}",
            self.addr(),
            self.ppn1(),
            self.ppn0(),
            self.is_readable(),
            self.is_writable(),
            self.is_executable(),
            self.is_valid()
        ))
    }
}
