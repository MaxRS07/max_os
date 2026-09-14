use crate::types::Brga;

/// Common api-facing gpu device
pub trait GpuDevice {
    type Error;

    /// Returns (width: u32, height: 32)
    fn dimensions(&self) -> (u32, u32);
    fn framebuffer(&mut self) -> &mut [Brga];
    fn flush(&mut self) -> Result<(), Self::Error>;
}
