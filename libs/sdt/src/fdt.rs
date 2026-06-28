use crate::{error::SdtError, stream::FdtStream};

/// representaion of a flattend device tree header
#[derive(Clone, Copy, Debug)]
pub struct FdtHeader {
    pub addr: usize,
    pub magic: u32,
    pub totalsize: u32,
    pub off_dt_struct: u32,
    pub off_dt_strings: u32,
    pub off_mem_rsvmap: u32,
    pub version: u32,
    pub last_comp_version: u32,
    pub boot_cpuid_phys: u32,
    pub size_dt_strings: u32,
    pub size_dt_struct: u32,
}
impl FdtHeader {
    pub fn from_raw_ptr(fdt_ptr: *const u8) -> core::result::Result<Self, SdtError> {
        let data = FdtHeader::read(fdt_ptr).unwrap();
        if let [
            magic,
            totalsize,
            off_dt_struct,
            off_dt_strings,
            off_mem_rsvmap,
            version,
            last_comp_version,
            boot_cpuid_phys,
            size_dt_strings,
            size_dt_struct,
            ..,
        ] = data
        {
            let magic = magic.swap_bytes();
            let totalsize = totalsize.swap_bytes();
            let off_dt_struct = off_dt_struct.swap_bytes();
            let off_dt_strings = off_dt_strings.swap_bytes();
            let off_mem_rsvmap = off_mem_rsvmap.swap_bytes();
            let version = version.swap_bytes();
            let last_comp_version = last_comp_version.swap_bytes();
            let boot_cpuid_phys = boot_cpuid_phys.swap_bytes();
            let size_dt_strings = size_dt_strings.swap_bytes();
            let size_dt_struct = size_dt_struct.swap_bytes();
            Ok(FdtHeader {
                addr: fdt_ptr as usize,
                magic,
                totalsize,
                off_dt_struct,
                off_dt_strings,
                off_mem_rsvmap,
                version,
                last_comp_version,
                boot_cpuid_phys,
                size_dt_strings,
                size_dt_struct,
            })
        } else {
            Err(SdtError::ParseError("One or more values are missing"))
        }
    }
    fn read(fdt_ptr: *const u8) -> core::result::Result<&'static [u32], SdtError> {
        unsafe {
            let val = (fdt_ptr as *const u32).read_volatile();
            let magic = val.swap_bytes();
            match magic == 0xD00DFEED {
                true => {
                    let data_ptr = fdt_ptr as *const u32;
                    let read_size = data_ptr.add(1).read_volatile().swap_bytes() as usize;
                    let parts = core::slice::from_raw_parts(data_ptr, read_size / 4); // read_size / 4 for u32 chunking 
                    Ok(parts)
                }
                false => Err(SdtError::ValidationError("Failed to validate FDT struct")),
            }
        }
    }
    pub fn get_stream(&self) -> FdtStream {
        FdtStream::new(self)
    }
}
