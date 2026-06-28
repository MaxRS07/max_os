use core_utils::sync::once::Once;
use log::{info, warn};
use sdt::{
    fdt::FdtHeader,
    mreg::MemoryRegion,
    stream::{FdtElement, FdtStream},
};

use crate::println;

pub static MEMORY_MAP: Once<MemoryMap> = Once::new();

#[derive(Clone, Copy, Default, Debug)]
pub struct MemoryMap {
    // System RAM and Boot Flash
    pub memory: MemoryRegion,
    pub flash: MemoryRegion,

    // Core Interruptors and Control
    pub clint: MemoryRegion,
    pub plic: MemoryRegion,
    pub pmu: MemoryRegion,

    // System Peripherals (Platform Bus / SoC Components)
    pub serial: MemoryRegion,
    pub rtc: MemoryRegion,
    pub fw_cfg: MemoryRegion,
    pub test: MemoryRegion, // QEMU test device (poweroff/reboot handles)

    // IO Virtualization Buses
    pub virtio_mmio: [MemoryRegion; 8],

    // PCI Subsystem Address Spaces
    pub pci_ecam: MemoryRegion,          // Configuration space
    pub pci_mmio_non_pref: MemoryRegion, // 32-bit BARs
    pub pci_mmio_pref: MemoryRegion,     // 64-bit prefetchable BARs
}

impl MemoryMap {
    ///
    /// # Safety
    pub unsafe fn from_stream(mut_stream: &mut FdtStream) -> Self {
        let mut map = MemoryMap::default();

        let mut node_stack: [&str; 8] = [""; 8];
        let mut depth = 0;
        let mut virtio_idx: usize = 0;
        let mut pci_reg_idx = 0;

        while let Some(element) = unsafe { mut_stream.next_element() } {
            match element {
                FdtElement::BeginNode { name } => {
                    let base_name = name.split('@').next().unwrap_or("");
                    if depth < node_stack.len() {
                        node_stack[depth] = base_name;
                        depth += 1;
                    }
                    pci_reg_idx = 0;
                }

                FdtElement::EndNode => {
                    if depth > 0 {
                        depth -= 1;
                        node_stack[depth] = ""; // Pop node off the stack
                    }
                }

                FdtElement::Property { name, .. } => {
                    if name != "reg" {
                        continue; // Skip non-memory-register tags safely
                    }

                    // Safely extract current node context from top of the stack
                    let current_node = if depth > 0 { node_stack[depth - 1] } else { "" };

                    // Catch potential parser panics when converting malformed blocks
                    let reg = match unsafe { MemoryRegion::from_fdt_element(element) } {
                        Some(r) => r,
                        None => {
                            warn!("Failed to parse reg for node: {}", current_node);
                            continue;
                        }
                    };

                    match current_node {
                        "flash" => map.flash = reg,
                        "memory" => map.memory = reg,
                        "clint" => map.clint = reg,
                        "plic" => map.plic = reg,
                        "pmu" => map.pmu = reg,
                        "serial" => map.serial = reg,
                        "rtc" => map.rtc = reg,
                        "fw-cfg" => map.fw_cfg = reg,
                        "test" => map.test = reg,
                        "virtio_mmio" => {
                            if virtio_idx < map.virtio_mmio.len() {
                                map.virtio_mmio[virtio_idx] = reg;
                                virtio_idx += 1;
                            }
                        }
                        "pci" | "pcie" => {
                            match pci_reg_idx {
                                0 => map.pci_ecam = reg,
                                1 => map.pci_mmio_non_pref = reg,
                                2 => map.pci_mmio_pref = reg,
                                _ => {}
                            }
                            pci_reg_idx += 1;
                        }
                        _ => {}
                    }
                }
            }
        }
        map
    }
}

pub fn init_static_map(fdt_ptr: *const u8) -> Result<MemoryMap, &'static str> {
    if let Ok(header) = FdtHeader::from_raw_ptr(fdt_ptr) {
        let mut stream = header.get_stream();
        unsafe {
            return Ok(MemoryMap::from_stream(&mut stream));
        }
    }
    Err("()")
}
