use crate::{
    fdt_header::FdtHeader,
    region::FDTRegion,
    stream::{FDTElement, FdtStream},
};
use alloc::{
    borrow::ToOwned,
    fmt::format,
    format,
    string::{String, ToString},
};
use collections::hashmap::{FnvBuildHasher, HashMap};
use log::{info, warn};
use sync::once::Once;

pub static GLOBAL_FDT: Once<FDT> = Once::new();

#[derive(Clone, Default, Debug)]
pub struct FDT<'a> {
    pub debug_mode: bool,
    // System RAM and Boot Flash
    pub memory: FDTRegion<'a>,
    pub flash: FDTRegion<'a>,

    // Core Interruptors and Control
    pub clint: FDTRegion<'a>,
    pub plic: FDTRegion<'a>,
    pub pmu: FDTRegion<'a>,

    // System Peripherals (Platform Bus / SoC Components)
    pub serial: FDTRegion<'a>,
    pub rtc: FDTRegion<'a>,
    pub fw_cfg: FDTRegion<'a>,
    pub test: FDTRegion<'a>, // QEMU test device (poweroff/reboot handles)

    // IO Virtualization Buses
    pub virtio_mmio: [FDTRegion<'a>; 8],

    // PCI Subsystem Address Spaces
    pub pci_ecam: FDTRegion<'a>,          // Configuration space
    pub pci_mmio_non_pref: FDTRegion<'a>, // 32-bit BARs
    pub pci_mmio_pref: FDTRegion<'a>,     // 64-bit prefetchable BARs
    element_map: HashMap<String, FDTElement>,
}

impl<'a> FDT<'a> {
    pub fn from_ptr(fdt_ptr: *const u8) -> Result<FDT<'a>, &'static str> {
        if let Ok(header) = FdtHeader::from_raw_ptr(fdt_ptr) {
            let mut stream = header.get_stream();
            unsafe { FDT::from_stream(&mut stream) }
        } else {
            Err("Couldn't resolve FDT from pointer")
        }
    }
    /// # Safety
    unsafe fn from_stream(mut_stream: &mut FdtStream) -> Result<Self, &'static str> {
        let mut map = FDT::default();

        let mut node_stack: [&str; 8] = [""; 8];
        let mut depth = 0;
        let mut virtio_idx: usize = 0;
        let mut pci_reg_idx = 0;

        while let Some(element) = unsafe { mut_stream.next_element() } {
            match element {
                FDTElement::BeginNode { name } => {
                    let base_name = name.split('@').next().unwrap_or("");
                    if depth < node_stack.len() {
                        node_stack[depth] = base_name;
                        depth += 1;
                    }
                    pci_reg_idx = 0;
                }

                FDTElement::Property {
                    name, value_ptr, ..
                } => {
                    // info!("{}", name);
                    let current_node = if depth > 0 { node_stack[depth - 1] } else { "" };
                    if current_node == "chosen" && name == "bootargs" {
                        map.debug_mode = unsafe { value_ptr.read() == 0x31 };
                    }

                    let path = Self::get_path(node_stack, depth, current_node, virtio_idx);

                    map.element_map
                        .insert(format!("{}/{}", path, name), element);
                }
                FDTElement::EndNode => {
                    let current_node = if depth > 0 { node_stack[depth - 1] } else { "" };
                    let path = Self::get_path(node_stack, depth, current_node, virtio_idx);
                    let reg_path = format!("{}/reg", path);
                    let region = map
                        .element_map
                        .get(reg_path)
                        .map(|val| unsafe {
                            FDTRegion::from_reg(current_node, val).unwrap_or(FDTRegion::new(
                                current_node,
                                0,
                                0,
                            ))
                        })
                        .unwrap_or(FDTRegion::new(current_node, 0, 0));
                    let format = format!("{:?}\n", region);
                    for char in format.bytes() {
                        unsafe { (0x1000_0000 as *mut u8).write_volatile(char) };
                    }
                    match current_node {
                        "flash" => map.flash = region,
                        "memory" => map.memory = region,
                        "clint" => map.clint = region,
                        "plic" => map.plic = region,
                        "pmu" => map.pmu = region,
                        "serial" => map.serial = region,
                        "rtc" => map.rtc = region,
                        "fw-cfg" => map.fw_cfg = region,
                        "test" => map.test = region,
                        "virtio_mmio" => {
                            if virtio_idx < map.virtio_mmio.len() {
                                map.virtio_mmio[virtio_idx] = region;
                                virtio_idx += 1;
                            }
                        }
                        "pci" | "pcie" => {
                            match pci_reg_idx {
                                0 => map.pci_ecam = region,
                                1 => map.pci_mmio_non_pref = region,
                                2 => map.pci_mmio_pref = region,
                                _ => {}
                            }
                            pci_reg_idx += 1;
                        }
                        _ => {}
                    }
                    if depth > 0 {
                        depth -= 1;
                        node_stack[depth] = ""; // Pop
                    }
                }
            }
        }
        Ok(map)
    }
    fn get_path(
        node_stack: [&str; 8],
        depth: usize,
        current_node: &str,
        virtio_idx: usize,
    ) -> String {
        let mut path = node_stack[..depth].join("/");

        if current_node == "virtio_mmio" {
            path = format!("{}/{}", path, virtio_idx);
        }
        path
    }

    pub fn get_element(&self, path: &str) -> Option<FDTElement> {
        self.element_map.get(path.to_string())
    }
    pub fn get_element_string(&self, path: &String) -> Option<FDTElement> {
        self.element_map.get(path.to_string())
    }
}

unsafe impl<'a> Send for FDT<'a> {}
unsafe impl<'a> Sync for FDT<'a> {}
