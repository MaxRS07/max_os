use core::{
    cell::OnceCell,
    sync::atomic::{
        Ordering::{Acquire, SeqCst},
        fence,
    },
};

use config::*;
use core::panic;
use log::info;
use log::warn;
use sdt::fdt::FDT;
use sync::once::OnceLock;

use virtio::VIRTIO_DEV_GPU;

pub mod config;
pub mod pci_bump;

pub const ECAM_BASE: u32 = 0x30000000;
pub const MMIO_LOW_BASE: u32 = 0x40000000; // Dynamic BAR allocation start pointer
pub const MMIO_LOW_LIMIT: u32 = 0x7FFFFFFF;

pub const VIRTIO_VENDOR_ID: u32 = 0x1AF4;
pub const VIRTIO_GPU_DEV_ID: u32 = 0x1050;

// Standard PCI Configuration Space Offsets
pub const PCI_REG_ID: u32 = 0x00; // Vendor ID (low 16) & Device ID (high 16)
pub const PCI_REG_COMMAND: u32 = 0x04; // Command register
pub const PCI_REG_BAR0: u32 = 0x10; // Base Address Register 0

// Command Register Bits
pub const PCI_CMD_IO_SPACE: u16 = (1 << 0);
pub const PCI_CMD_MEM_SPACE: u16 = (1 << 1);
pub const PCI_CMD_BUS_MASTER: u16 = (1 << 2);

static ALLOCATOR: OnceLock<pci_bump::PciAllocator> = OnceLock::new();

pub fn init(map: FDT) {
    let pci_alloc = pci_bump::PciAllocator::new(map);
    ALLOCATOR.init(|| pci_alloc);
    fence(SeqCst);
    info!("PCI allocator initialized")
}

pub fn probe_devices(ecam: usize) -> Option<(u8, u8)> {
    for bus in 0..=255u8 {
        for dev in 0..=31u8 {
            let vendor = pci_read::<u16>(ecam, bus, dev, 0, PCI_CFG_VENDOR_ID);
            if vendor == PCI_VENDOR_NONE {
                continue;
            }

            let device_id = pci_read::<u16>(ecam, bus, dev, 0, PCI_CFG_DEVICE_ID);

            if vendor as u32 == VIRTIO_VENDOR_ID && device_id == VIRTIO_DEV_GPU {
                return Some((bus, dev));
            }
        }
    }
    None
}
pub fn ping_virtio(ecam: usize, bus: u8, dev: u8, func: u8, offset: u16) {
    if unsafe { !ALLOCATOR.is_executed() } {
        return;
    };
    let vendor = pci_read::<u16>(ecam, bus, dev, 0, PCI_CFG_VENDOR_ID);
    // 0xFFF is an empty vendor marker
    if vendor == PCI_VENDOR_NONE {
        return;
    }

    let id = pci_read::<u16>(ecam, bus, dev, 0, 0x02);
    let header_type = pci_read::<u8>(ecam, bus, dev, 0, PCI_CFG_HEADER_TYPE) & 0x7F;

    if header_type != 0x00 {
        return;
    }

    let cmd = pci_read::<u16>(ecam, bus, dev, 0x00, PCI_CFG_COMMAND);
    pci_write::<u16>(ecam, bus, dev, func, offset, cmd | 0x6);

    let mut bar_idx = 0;
    while bar_idx < 6 {
        let offset = PCI_CFG_BAR0 + (bar_idx * 4) as u16; // iter through bars
        let bar = pci_read::<u32>(ecam, bus, dev, 0, offset);

        if bar & 0x1 != 0 {
            bar_idx += 1; // I/O BAR, skip
            continue;
        }

        let is_64bit = (bar >> 1) & 0x3 == 0x2;

        // get size
        pci_write::<u32>(ecam, bus, dev, 0, offset, 0xFFFFFFFF);
        let size_mask = pci_read::<u32>(ecam, bus, dev, 0, offset);
        pci_write::<u32>(ecam, bus, dev, 0, offset, bar);
        let size = (!(size_mask & PCI_BAR_ADDR_MASK)).wrapping_add(1) as usize;

        if size > 0
            && let Some(alloc) = ALLOCATOR.get_mut_ptr()
            && let Some(addr) = unsafe { (*alloc).alloc_non_pref(size) }
        {
            pci_write::<u32>(ecam, bus, dev, 0, offset, addr as u32);
        } else {
            warn!("Failed to allocate memory")
        }

        bar_idx += if is_64bit { 2 } else { 1 }; // 64 bit bars consume two slots
    }
}

/// encode instruction into u32
fn cfg_addr(ecam: usize, bus: u8, dev: u8, func: u8, offset: u16) -> usize {
    ecam | ((bus as usize) << 20)
        | ((dev as usize) << 15)
        | ((func as usize) << 12)
        | ((offset as usize) & 0xFFF) // mask to 12 bits
}

pub fn pci_read<T>(ecam: usize, bus: u8, dev: u8, func: u8, offset: u16) -> T {
    unsafe { (cfg_addr(ecam, bus, dev, func, offset) as *const T).read_volatile() }
}

pub fn pci_write<T>(ecam: usize, bus: u8, dev: u8, func: u8, offset: u16, val: T) {
    unsafe { (cfg_addr(ecam, bus, dev, func, offset) as *mut T).write_volatile(val) }
}
