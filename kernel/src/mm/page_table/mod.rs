use core::{arch::asm, panic, ptr::addr_of_mut};

use log::debug;

use mem::{
    error::MemoryError,
    page_table::{page_alloc::Pager, table::Table},
};
use sdt::{fdt::GLOBAL_FDT, region::FDTRegion};

use crate::{arch::riscv::csr::Csr::SATP, mm::heap::PAGE_ALLOCATOR};

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
            let mut_root = unsafe { &mut *root_ptr };
            // RAM
            map_region_identity(
                mut_root,
                PAGE_ALLOCATOR.wait(),
                &fdt.memory,
                RWX | Table::MEGAPAGE,
            );
            debug!("Mapped RAM");
            // CLINT
            map_region_identity(mut_root, PAGE_ALLOCATOR.wait(), &fdt.clint, RW);
            debug!("Mapped CLINT");
            // PLIC
            map_region_identity(mut_root, PAGE_ALLOCATOR.wait(), &fdt.plic, RW);
            debug!("Mapped PLIC");
            // MMIO
            map_mmio_identity(mut_root, PAGE_ALLOCATOR.wait(), &fdt.virtio_mmio, RW);
            debug!("Mapped MMIO");
            // Test
            map_region_identity(mut_root, PAGE_ALLOCATOR.wait(), &fdt.test, RW);
            debug!("Mapped Test");
            // RTC
            map_region_identity(mut_root, PAGE_ALLOCATOR.wait(), &fdt.rtc, RW);
            debug!("Mapped RTC");
            // serial (uart)
            map_region_identity(mut_root, PAGE_ALLOCATOR.wait(), &fdt.serial, RW);
            debug!("Mapped Serial");
            // fw-cfg
            map_region_identity(mut_root, PAGE_ALLOCATOR.wait(), &fdt.fw_cfg, RW);
            debug!("Mapped FW-CFG");
        }
        Ok(())
    }
}

/* Low level FDT map helpers */
/// Maps a virtual address to its respective contiguous physical region. For non-contiguous region mapping use `map_region`
pub fn map_region_identity(
    table: &mut Table,
    page_allocator: &dyn Pager,
    region: &FDTRegion,
    flags: usize,
) -> Result<(), MemoryError> {
    let virt_addr = region.base_address;
    let offset_size = if flags & Table::MEGAPAGE != 0 {
        0 // MEGATABLE_SIZE
    } else {
        0 // LEAF_SIZE
    };
    let page_count = region.size.div_ceil(offset_size);
    for i in 0..page_count {
        let offset = i * offset_size;
        let vaddr = virt_addr + offset as usize;
        let paddr = (region.base_address + offset) as usize;

        table.map(PAGE_ALLOCATOR.wait(), vaddr, paddr, flags)?;
    }

    Ok(())
}

pub fn map_mmio_identity(
    table: &mut Table,
    page_allocator: &dyn Pager,
    mmio_slots: &[FDTRegion],
    flags: usize,
) -> Result<(), MemoryError> {
    for slot in mmio_slots {
        map_region_identity(table, page_allocator, slot, flags)?;
    }
    Ok(())
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
