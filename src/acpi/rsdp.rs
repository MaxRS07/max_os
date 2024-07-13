use core::{error, fmt::Error};

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

use crate::{println, utils::mem_utils::*};

pub const RANGES: [[u64; 2]; 6] = [
    [0x0000000000001000u64, 0x0000000000015000u64], //14000
    [0x00000000000a0000u64, 0x00000000000c0000u64], //20000
    [0x0000000000200000u64, 0x0000000000215000u64], //15000
    [0x0000000000215000u64, 0x0000000000217000u64], //2000
    [0x0000010000000000u64, 0x0000010000001000u64], //1000
    [0x0000010000002000u64, 0x0000010000202000u64], //200000
];

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RSDP {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oemid: [u8; 6],
    pub revision: u8,
    pub rsdt_addr: u32,
}
impl RSDP {
    ///loads and returns rsdp table if found, else returns None
    pub fn load_rsdp() -> Option<&'static RSDP> {
        let rsd_ptr: &[u8; 8] = b"RSD PTR ";
        let start3 = 0x200000;
        let end3 = 0x202000;

        unsafe {
            let rsdp_addr = search(start3, end3, rsd_ptr, 8);
            match rsdp_addr {
                Some(value) => {
                    let rsdp = &*(value as *const RSDP);
                    if rsdp.verify() {
                        return Some(rsdp);
                    }
                }
                None => (),
            }
        }
        None
    }
    pub fn verify(&self) -> bool {
        self.signature == *b"RSD PTR " && verify_checksum(self)
    }
}
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct XSDP {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oemid: [u8; 6],
    pub revision: u8,
    pub rsdt_addr: u32,

    pub len: u32,
    pub xsdt_addr: u64,
    pub extended_checksum: u8,
    pub reserved: [u8; 3],
}
impl XSDP {
    ///loads and returns xsdp table if found, else returns None
    pub fn load_xsdp(map: &MemoryMap) -> Option<XSDP> {
        const RSD_PTR: &[u8; 8] = b"RSD PTR ";
        for memory_region in map.iter() {
            if memory_region.region_type == MemoryRegionType::Usable {
                let base_address = memory_region.range.start_frame_number;
                let end_address = memory_region.range.end_frame_number;

                for addr in (base_address..end_address).step_by(16) {
                    let slice =
                        unsafe { core::slice::from_raw_parts(addr as *const u8, RSD_PTR.len()) };
                    if slice == RSD_PTR {
                        let xsdp = unsafe { *(addr as *const Self) };
                        return Some(xsdp);
                    }
                }
            }
        }
        None
    }
    pub fn verify(&self) -> bool {
        self.signature == *b"RSD PTR " // && self.oemid == b"BOCHS " // && verify_checksum(self)
    }
}
