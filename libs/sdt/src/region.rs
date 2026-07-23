use core::fmt::Debug;

use mem::pointerator::Pointerator;

use crate::stream::FDTElement;

#[derive(Clone, Default)]
#[repr(C)]
/// Represents a memeory region in the FDT, parsed from reg property
pub struct FDTRegion<'a> {
    pub name: &'a str,
    pub base_address: usize,
    pub size: usize,
}

impl<'a> FDTRegion<'a> {
    pub fn as_ptr<T>(&self) -> *mut T {
        self.base_address as *mut T
    }
    pub fn contains(&self, addr: usize) -> bool {
        addr >= self.base_address && addr < self.base_address + self.size
    }
    pub fn new(name: &'a str, base_address: usize, size: usize) -> Self {
        Self {
            name,
            base_address,
            size,
        }
    }
    /// ## Args
    /// `ptr`: address of the start tag of the `MemoryRegion` (0x00000003).
    ///
    /// ## Returns
    /// `Ok(MemoryRegion)`\
    /// `SdtError`: if validation fails or YOU put the wrong element here.
    ///
    /// ## Safety
    ///
    pub unsafe fn from_reg(name: &'a str, el: FDTElement) -> Option<Self> {
        match el {
            FDTElement::BeginNode { name: _ } | FDTElement::EndNode => None,
            FDTElement::Property {
                name: _,
                value_ptr,
                len,
            } => unsafe {
                let mut tag_data = Pointerator::from_ptr(value_ptr, len);
                match len {
                    8 => {
                        let base_address: u32 = tag_data.next().unwrap_or(0);
                        let size: u32 = tag_data.next().unwrap_or(0);
                        Some(FDTRegion {
                            name,
                            base_address: u32::from_be(base_address) as usize,
                            size: u32::from_be(size) as usize,
                        })
                    }
                    16 => {
                        let base_hi: u32 = tag_data.next().unwrap_or(0);
                        let base_lo: u32 = tag_data.next().unwrap_or(0);
                        let size_hi: u32 = tag_data.next().unwrap_or(0);
                        let size_lo: u32 = tag_data.next().unwrap_or(0);

                        let base_address =
                            ((u32::from_be(base_hi) as u64) << 32) | u32::from_be(base_lo) as u64;
                        let size =
                            ((u32::from_be(size_hi) as u64) << 32) | u32::from_be(size_lo) as u64;

                        Some(FDTRegion {
                            name,
                            base_address: base_address as usize,
                            size: size as usize,
                        })
                    }
                    _ => None,
                }
            },
        }
    }
}

impl<'a> Debug for FDTRegion<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FDTRegion")
            .field("name", &self.name)
            .field("base_address", &format_args!("0x{:x}", self.base_address))
            .field("size", &format_args!("0x{:x}", self.size))
            .finish()
    }
}
