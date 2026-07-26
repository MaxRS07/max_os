use core::{
    arch::asm,
    panic,
    ptr::{addr_of, addr_of_mut},
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

use alloc::boxed::Box;
use sdt::{fdt::GLOBAL_FDT, stream::FDTElement};
use sync::once::Once;

use crate::{
    arch::riscv::csr::Csr::SATP,
    console::writer::println,
    mm::heap::page_table::table::{Table, TableEntry},
    println,
};

pub mod pdpt;
pub mod table;

static mut ROOT_TABLE: Table = Table::new();

/// Creates global page table root, maps 0x8000_0000 to itself
pub fn init_root_table() {
    unsafe {
        let root_ptr = addr_of_mut!(ROOT_TABLE);
        let root_addr = root_ptr.addr();
        println!("Created root table at 0x{:x}", root_addr);
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
        println!("Mapping regions");
        // identity mappings
        if let Some(fdt) = GLOBAL_FDT.get() {
            // RAM
            let _ = (*root_ptr).map_region_identity(&fdt.memory, RWX | Table::MEGAPAGE);
            println!("Mapped RAM");
            // CLINT
            let _ = (*root_ptr).map_region_identity(&fdt.clint, RW);
            println!("Mapped CLINT");
            // PLIC
            let _ = (*root_ptr).map_region_identity(&fdt.plic, RW);
            println!("Mapped PLIC");
            // MMIO
            let _ = (*root_ptr).map_mmio_identity(&fdt.virtio_mmio, RW);
            println!("Mapped MMIO");
            // Test
            let _ = (*root_ptr).map_region_identity(&fdt.test, RW);
            println!("Mapped Test");
            // RTC
            let _ = (*root_ptr).map_region_identity(&fdt.rtc, RW);
            println!("Mapped RTC");
            // serial (uart)
            let _ = (*root_ptr).map_region_identity(&fdt.serial, RW);
            println!("Mapped Serial");
            // fw-cfg
            let _ = (*root_ptr).map_region_identity(&fdt.fw_cfg, RW);
            println!("Mapped FW-CFG");
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
