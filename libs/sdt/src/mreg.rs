use core_utils::pointerator::Pointerator;

use crate::stream::FdtElement;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
#[derive(Default)]
pub struct MemoryRegion {
    pub base_address: usize,
    pub size: usize,
}

impl MemoryRegion {
    pub fn as_ptr<T>(&self) -> *mut T {
        self.base_address as *mut T
    }
    pub fn contains(&self, addr: usize) -> bool {
        addr >= self.base_address && addr < self.base_address + self.size
    }
    pub fn new(base_address: usize, size: usize) -> Self {
        Self { base_address, size }
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
    pub unsafe fn from_fdt_element(el: FdtElement) -> Option<Self> {
        match el {
            FdtElement::BeginNode { name: _ } | FdtElement::EndNode => None,
            FdtElement::Property {
                name: _,
                value_ptr,
                len,
            } => unsafe {
                let mut tag_data = Pointerator::new(value_ptr, len);
                match len {
                    8 => {
                        let base_address: u32 = tag_data.next().unwrap_or(0);
                        let size: u32 = tag_data.next().unwrap_or(0);
                        Some(MemoryRegion {
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

                        Some(MemoryRegion {
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
