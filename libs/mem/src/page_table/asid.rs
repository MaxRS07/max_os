// TODO: THREAD SAFETY: Exchanges need to be atomic; Implement teardown

use collections::bitmap::Bitmap;

/// This is the maximum ASID value on SV32
pub const MAX_ASID: u16 = 512;
/// Number of bytes in the bitmap
const ASID_BYTES: u16 = MAX_ASID / 8;

/// Represents a 32 bit ASID with `0 < Self::value < 512`
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

pub type ASIDBitmapSV32 = Bitmap<[u64; 8]>;

trait ASIDAllocator {
    /// Retuns the next unused ASID
    fn alloc_id(&mut self) -> Option<ASID>;
    /// Frees an ASID
    fn free_id(&mut self, id: ASID);
}

impl ASIDAllocator for ASIDBitmapSV32 {
    fn alloc_id(&mut self) -> Option<ASID> {
        self.next_free().map(|bit| {
            self.set_used(bit);
            return ASID(bit as u16);
        })
    }
    fn free_id(&mut self, asid: ASID) {
        self.set_free(asid.0 as usize);
    }
}
