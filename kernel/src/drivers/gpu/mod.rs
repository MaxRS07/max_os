use log::{info, warn};
use sdt::fdt::FDT;

pub mod error;
pub mod virtio;

/// GPU initialization for VirtIO device only
/// TODO: Detect hardware and initialize
pub fn init(fdt: &FDT, mmio_idx: usize) {
    info!("Initializing MMIO GPU on: {}", mmio_idx);
    if let Some(virtio_gpu) = virtio::VirtioGpu::from_mmio(fdt, mmio_idx) {
        info!("Created VirtIO GpuDevice")
    } else {
        warn!("Failed to create VirtIO GpuDevice")
    }
}
