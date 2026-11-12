use core::{arch::asm, panic, ptr::addr_of_mut};

use log::debug;

use mem::{
    error::MemoryError,
    page_table::{
        page_alloc::Pager,
        table::{PageSize, Table},
    },
};
use sdt::{fdt::GLOBAL_FDT, region::FDTRegion};

use crate::{arch::riscv::csr::Csr::SATP, mm::heap::PAGE_ALLOCATOR};

// read + write
const RW: usize = 0b111;

// read + execute
const RX: usize = 0b1011;

// read + write + execute
const RWX: usize = 0b1111;

pub fn map_boot_pages(root_ptr: *mut Table) -> Result<(), MemoryError> {
    debug!("Mapping regions");
    // identity mappings
    if let Some(fdt) = GLOBAL_FDT.get() {
        let mut_root = unsafe { &mut *root_ptr };
        // RAM
        map_region_identity(
            mut_root,
            PAGE_ALLOCATOR.wait(),
            &fdt.memory,
            PageSize::Gigapage,
            RWX,
        )?;
        debug!("Mapped RAM");
        // CLINT
        map_region_identity(
            mut_root,
            PAGE_ALLOCATOR.wait(),
            &fdt.clint,
            PageSize::Single,
            RW,
        )?;
        debug!("Mapped CLINT");
        // PLIC
        map_region_identity(
            mut_root,
            PAGE_ALLOCATOR.wait(),
            &fdt.plic,
            PageSize::Single,
            RW,
        )?;
        debug!("Mapped PLIC");
        // MMIO
        map_mmio_identity(mut_root, PAGE_ALLOCATOR.wait(), &fdt.virtio_mmio, RW)?;
        debug!("Mapped MMIO");
        // Test
        map_region_identity(
            mut_root,
            PAGE_ALLOCATOR.wait(),
            &fdt.test,
            PageSize::Single,
            RW,
        )?;
        debug!("Mapped Test");
        // RTC
        map_region_identity(
            mut_root,
            PAGE_ALLOCATOR.wait(),
            &fdt.rtc,
            PageSize::Single,
            RW,
        )?;
        debug!("Mapped RTC");
        // serial (uart)
        map_region_identity(
            mut_root,
            PAGE_ALLOCATOR.wait(),
            &fdt.serial,
            PageSize::Single,
            RW,
        )?;
        debug!("Mapped Serial");
        // fw-cfg
        map_region_identity(
            mut_root,
            PAGE_ALLOCATOR.wait(),
            &fdt.fw_cfg,
            PageSize::Single,
            RW,
        )?;
        debug!("Mapped FW-CFG");
    }
    Ok(())
}

/* Low level FDT map helpers */
/// Maps a virtual address to its respective contiguous physical region. For non-contiguous region mapping use `map_region`
pub fn map_region_identity(
    table: &mut Table,
    page_allocator: &dyn Pager,
    region: &FDTRegion,
    page_size: PageSize,
    flags: usize,
) -> Result<(), MemoryError> {
    let virt_addr = region.base_address;
    let offset_size = page_size as usize;
    let page_count = region.size.div_ceil(offset_size);
    for i in 0..page_count {
        let offset = i * offset_size;
        let vaddr = virt_addr + offset as usize;
        let paddr = vaddr; // readability

        table.map(page_allocator, vaddr, paddr, flags, page_size)?;
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
        map_region_identity(table, page_allocator, slot, PageSize::Single, flags)?;
    }
    Ok(())
}
