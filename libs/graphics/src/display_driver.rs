use core::u32;

use crate::{error::GpuError, resource::GpuResouce};

/// Common api-facing gpu device
pub trait DisplayDriver {
    /* Resource Management */
    /// creates a 2d resource with the provided width and height, returning the resource id
    fn create_resource(&mut self, width: usize, height: usize) -> Result<u32, GpuError>;
    /// Destroys the resource with the given `resource_id` and deallocates the backing.
    /// Returns [`Ok`] containing the size of the freed backing in bytes
    fn destroy_resource(&mut self, resource_id: u32) -> Result<usize, GpuError>;
    fn get_resource(&self, resource_id: u32) -> Option<GpuResouce>;

    /*  */
    fn dimensions(&self) -> (usize, usize);
    fn flush(&mut self) -> Result<(), GpuError>;
    fn flip(&mut self) -> Result<(), GpuError>;
}
