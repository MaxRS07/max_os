use core::u32;

use crate::{bgra::Brga, error::GpuDeviceError};

/// Common api-facing gpu device
pub trait GpuDevice {
    fn create_resource(
        &mut self,
        resource_id: u32,
        width: u32,
        height: u32,
    ) -> Result<u32, GpuDeviceError>;
    fn destroy_resource(&mut self, resource_id: u32) -> Result<(), GpuDeviceError>;
    /// Returns (width: u32, height: 32)
    fn dimensions(&self) -> (u32, u32);
    fn framebuffer(&mut self) -> &mut [Brga];
    fn flush(&mut self) -> Result<(), GpuDeviceError>;

    fn flip(&mut self) -> Result<(), GpuDeviceError>;
}
