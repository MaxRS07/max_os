use alloc::sync::Arc;
use fdt::tree::FDT;
use graphics::device::GpuDevice;
use log::{info, warn};
use sync::{mutex::Mutex, oncelock::OnceLock};

use crate::drivers::gpu::virtio::VirtioGpu;

pub mod virtio;

pub static GPU_DEVICE: OnceLock<Mutex<Arc<VirtioGpu>>> = OnceLock::new();

/// GPU initialization for VirtIO device only
/// TODO: Detect hardware and initialize
pub fn init(fdt: &FDT, mmio_idx: usize) {
    info!("Initializing MMIO GPU on: {}", mmio_idx);
    match virtio::VirtioGpu::new(fdt, mmio_idx) {
        Ok(gpu) => {
            GPU_DEVICE.set(Mutex::new(Arc::new(gpu)));
            info!("Created VirtIO GpuDevice")
        }
        Err(error) => {
            warn!("Failed to create VirtIO GpuDevice: {error}")
        }
    }
}
