use alloc::sync::Arc;
use graphics::device::GpuDevice;
use log::{info, warn};
use sdt::fdt::FDT;
use sync::{mutex::Mutex, oncelock::OnceLock};

use crate::drivers::gpu::virtio::VirtioGpu;

pub mod error;
pub mod virtio;

pub static GPU_DEVICE: OnceLock<Mutex<Arc<VirtioGpu>>> = OnceLock::new();

/// GPU initialization for VirtIO device only
/// TODO: Detect hardware and initialize
pub fn init(fdt: &FDT, mmio_idx: usize) {
    info!("Initializing MMIO GPU on: {}", mmio_idx);
    if let Some(virtio_gpu) = virtio::VirtioGpu::from_mmio(fdt, mmio_idx) {
        GPU_DEVICE.set(Mutex::new(Arc::new(virtio_gpu)));
        info!("Created VirtIO GpuDevice")
    } else {
        warn!("Failed to create VirtIO GpuDevice")
    }
}
