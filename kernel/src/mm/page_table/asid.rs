// TODO: THREAD SAFETY: Exchanges need to be atomic; Implement teardown
use core::panicking::panic;

use crate::mm::page_table::asid;

/// This is the maximum ASID value on SV32
pub const MAX_ASID: u16 = 512;
/// Number of bytes in the bitmap
const ASID_BYTES: u16 = MAX_ASID / 8;

/// Represents a 32 bit ASID with `0 < Self::value < 512`
pub struct ASID(u16);
impl ASID {
    /// Creates a new ASID from a value
    pub fn new(value: u16) -> Self {
        if value > MAX_ASID {
            panic!("ASID cannot be greater than 512 on SV32")
        }
        ASID(value)
    }
    /// Returns the ASID value
    pub fn value(&self) -> u16 {
        self.0
    }
}
trait ASIDAllocator {
    /// Retuns the next unused ASID
    fn alloc_id(&mut self) -> Option<u16>;
    /// Frees an ASID
    fn free_id(&mut self, id: u16);
}
pub struct ASIDBitmap {
    asids: [u8; ASID_BYTES],
    /// last free id is cached here
    hint: u16,
}
impl ASIDBitmap {
    /// This id is reserved for the kernel process. It is always used
    pub const RESERVED: u16 = 0;

    pub const fn new() -> Self {
        let mut asids = [0u8; 512];
        asids[0] = 1;
        Self { asids, hint: 0 }
    }
    /// returns the next unused ASID
    /// TODO: Implement recycling
    pub fn alloc_id(&mut self) -> Option<u16> {
        for i in self.hint..(self.hint + MAX_ASID - 1) {
            let id = i % MAX_ASID;
            if !self.acid_used(i) {
                self.hint = i + 1;
                self.use_asid(id);
                return Some(i);
            }
        }
        return None;
    }
    pub fn free_id(&mut self, id: u16) {
        if id == Self::RESERVED {
            panic("Attempted to free kernel page")
        }
        self.hint = id;
        self.free_asid(id);
    }
    /// returns true if the given ASID is used by a process, false if the ASID is free
    fn acid_used(&self, id: u16) -> bool {
        let byte = id / ASID_BYTES;
        let bit = id % 8;
        self.asids[byte] & (1 << bit)
    }
    fn use_asid(&mut self, id: u16) {
        let byte = id / ASID_BYTES;
        let bit = id % 8;
        self.asids[byte] |= (1 << bit)
    }
    fn free_asid(&mut self, id: u16) {
        let byte = id / ASID_BYTES;
        let bit = id % 8;
        self.asids[byte] &= !(1 << bit)
    }
}

impl ASIDAllocator for ASIDBitmap {
    fn alloc_id(&mut self) -> Option<u16> {
        self.alloc_id()
    }
    fn free_id(&mut self, id: u16) {
        self.free_id(id);
    }
}
