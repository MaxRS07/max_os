use core::{arch::asm, panic, ptr::addr_of_mut};

use log::debug;

use sdt::fdt::GLOBAL_FDT;

use crate::{arch::riscv::csr::Csr::SATP, mm::heap::page_table::table::Table};

pub mod address_space;
pub mod table;
pub mod table_entry;

static mut ROOT_TABLE: Table = Table::new();

/// Creates global page table root, maps 0x8000_0000 to itself
pub fn init_root_table() {
    unsafe {
        let root_ptr = &raw mut ROOT_TABLE;
        let root_addr = root_ptr.addr();
        debug!("Created root table at 0x{:x}", root_addr);
        match map_boot_pages(root_ptr) {
            Ok(()) => pack_satp(root_addr),
            Err(error) => panic!("{}", error),
        }
    }
}

// read + write
const RW: usize = 0b111;

// read + execute
const RX: usize = 0b1011;

// read + write + execute
const RWX: usize = 0b1111;

fn map_boot_pages(root_ptr: *mut Table) -> Result<(), &'static str> {
    unsafe {
        debug!("Mapping regions");
        // identity mappings
        if let Some(fdt) = GLOBAL_FDT.get() {
            // RAM
            let _ = (*root_ptr).map_region_identity(&fdt.memory, RWX | Table::MEGAPAGE);
            debug!("Mapped RAM");
            // CLINT
            let _ = (*root_ptr).map_region_identity(&fdt.clint, RW);
            debug!("Mapped CLINT");
            // PLIC
            let _ = (*root_ptr).map_region_identity(&fdt.plic, RW);
            debug!("Mapped PLIC");
            // MMIO
            let _ = (*root_ptr).map_mmio_identity(&fdt.virtio_mmio, RW);
            debug!("Mapped MMIO");
            // Test
            let _ = (*root_ptr).map_region_identity(&fdt.test, RW);
            debug!("Mapped Test");
            // RTC
            let _ = (*root_ptr).map_region_identity(&fdt.rtc, RW);
            debug!("Mapped RTC");
            // serial (uart)
            let _ = (*root_ptr).map_region_identity(&fdt.serial, RW);
            debug!("Mapped Serial");
            // fw-cfg
            let _ = (*root_ptr).map_region_identity(&fdt.fw_cfg, RW);
            debug!("Mapped FW-CFG");
        }
        Ok(())
    }
}

unsafe fn pack_satp(root_addr: usize) {
    let addr_mask = 0x3FFFFFu32;
    let mut satp_value = 0;
    satp_value |= 1 << 31; // select sv32 mode

    let addr = (root_addr >> 12) as u32;
    satp_value |= addr_mask & addr;

    unsafe {
        SATP.write(satp_value as usize);
        asm!(
            "sfence.vma zero, zero",
            "fence rw, rw",
            "fence.i",
            options(nostack)
        );
    }
}
