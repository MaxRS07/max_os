use sdt::fdt::FDT;

pub mod virtio;

pub fn init(fdt: &FDT, mmio_idx: usize) {
    if let Some(net) = virtio::VirtioNet::from_mmio(fdt, mmio_idx, cfg_flags, max_pairs) {
        net.send
    }
}
