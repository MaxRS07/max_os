use crate::{println, utils::mem_utils::*};

pub const RANGES: [[u64; 2]; 5] = [
    [0x0000000000001000u64, 0x0000000000015000u64],
    [0x00000000000a0000u64, 0x00000000000c0000u64],
    [0x0000000000200000u64, 0x0000000000202000u64],
    [0x0000010000000000u64, 0x0000010000001000u64],
    [0x0000010000002000u64, 0x0000010000202000u64],
];

/// | Value | Address Space                                 
/// |------ | ---------------------------------------------
/// |0     | System Memory                               
/// |1     | System I/O                                  
/// |2     | PCI Configuration Space                     
/// |3     | Embedded Controller                         
/// |4     | System Management Bus                       
/// |5     | System CMOS                                 
/// |6     | PCI Device BAR Target                       
/// |7     | Intelligent Platform Management Infrastructure
/// |8     | General Purpose I/O                         
/// |9     | Generic Serial Bus                          
/// |0x0A  | Platform Communication Channel              
/// |0x0B to 0x7F | Reserved                             
/// |0x80 to 0xFF | OEM Defined  
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct GenericAddressStructure {
    addr_space: u8,
    bit_width: u8,
    bit_offset: u8,
    access_size: u8,
    addr: u64,
}
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ACPISDTHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oemid: [u8; 6],
    pub oemtable_id: [u8; 6],
    pub oemrevision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}
#[repr(C)]
pub struct RSDT {
    pub header: ACPISDTHeader,
    pub pointer_to_other_sdt: u32,
}
impl RSDT {
    pub fn load_rsdt() -> Option<&'static RSDT> {
        let facp_ptr: &[u8; 4] = b"FACP";

        unsafe {
            for range in RANGES {
                let rsdt_addr = search(range[0], range[1], facp_ptr, 1);
                if let Some(rsdt_addr) = rsdt_addr {
                    let rsdt = &*(rsdt_addr as *const RSDT);
                    return Some(rsdt);
                }
            }
        }
        None
    }
}
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FADT {
    header: ACPISDTHeader,
    firmware_ctrl: u32,
    uint32_t: u32,

    // field used in ACPI 1.0; no longer in use, for compatibility only
    reserved: u8,

    preferred_power_management_profile: u8,
    sci_interrupt: u16,
    smi_command_port: u32,
    acpi_enable: u8,
    acpi_disable: u8,
    s4bios_req: u8,
    pstate_control: u8,
    pm1a_event_block: u32,
    pm1b_event_block: u32,
    pm1a_control_block: u32,
    pm1b_control_block: u32,
    pm2_control_block: u32,
    pmtimer_block: u32,
    gpe0_block: u32,
    gpe1_block: u32,
    pm1_event_length: u8,
    pm1_control_length: u8,
    pm2_control_length: u8,
    pmtimer_length: u8,
    gpe0_length: u8,
    gpe1_length: u8,
    gpe1_base: u8,
    cstate_control: u8,
    worst_c2_latency: u16,
    worst_c3_latency: u16,
    flush_size: u16,
    flush_stride: u16,
    duty_offset: u8,
    duty_width: u8,
    day_alarm: u8,
    month_alarm: u8,
    century: u8,

    // reserved in ACPI 1.0; used since ACPI 2.0+
    boot_architecture_flags: u16,

    reserved2: u8,
    flags: u32,

    // 12 byte structure; see below for details
    reset_reg: GenericAddressStructure,

    reset_value: u8,
    reserved3: [u8; 3],

    // 64bit pointers - Available on ACPI 2.0+
    x_firmware_control: u64,
    x_dsdt: u64,

    x_pm1a_event_block: GenericAddressStructure,
    x_pm1b_event_block: GenericAddressStructure,
    x_pm1a_control_block: GenericAddressStructure,
    x_pm1b_control_block: GenericAddressStructure,
    x_pm2_control_block: GenericAddressStructure,
    x_pmtimer_block: GenericAddressStructure,
    x_gpe0_block: GenericAddressStructure,
    x_gpe1_block: GenericAddressStructure,
}
impl FADT {
    pub fn load_fadt(addr: u64) -> Option<&'static Self> {
        let fadt = addr as *const Self;
        unsafe { return Some(&*fadt) }
    }
}
