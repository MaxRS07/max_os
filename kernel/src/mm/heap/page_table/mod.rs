use core::{
    arch::asm,
    panic,
    ptr::{addr_of, addr_of_mut},
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

use alloc::boxed::Box;
use sdt::fdt::GLOB_FDT;
use sync::once::Once;

use crate::{
    arch::riscv::csr::Csr::SATP,
    console::writer::println,
    mm::heap::page_table::map::{Table, TableEntry},
    println,
};

pub mod map;
pub mod pdpt;

static mut ROOT_TABLE: Table = Table::new();

/// Creates global page table root, maps 0x8000_0000 to itself
pub fn init_root_table() {
    unsafe {
        let root_ptr = addr_of_mut!(ROOT_TABLE);
        let root_addr = root_ptr.addr();
        println!("Created root table at 0x{:0x}", root_addr);
        match map_boot_pages(root_ptr) {
            Ok(()) => {
                println("packing satp");
                pack_satp(root_addr);
            }
            Err(error) => panic!("{}", error),
        }
    }
}

fn map_boot_pages(root_ptr: *mut Table) -> Result<(), &'static str> {
    unsafe {
        let _ = (*root_ptr).map(
            0x8000_0000,
            0x8000_0000,
            0b1011 | Table::MEGAPAGE, // 4MiB boot megatable on 0x8000_0000, RE on
        );
        // identity mappings
        if let Some(fdt) = GLOB_FDT.get() {
            // UART0
            let uart0 = 0x1000_0000u32;
            let _ = (*root_ptr).map(uart0, uart0, 0b101);

            // CLINT
            let clint = fdt.clint.base_address as u32;
            let _ = (*root_ptr).map(clint, clint, 0b111);

            // PLIC
            let plic = fdt.plic.base_address as u32;
            let _ = (*root_ptr).map(plic, plic, 0b111);
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
        asm!("sfence.vma");
    }
}
