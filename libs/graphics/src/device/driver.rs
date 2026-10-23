use crate::{
    device::{
        backing::{Backing, BackingMode},
        display::DisplayInfo,
        resource::{ResourceDesc, ResourceHandle},
    },
    error::DisplayError,
    primitives::rect::ScreenRect,
};

pub trait DisplayDriver {
    fn info(&self) -> DisplayInfo;
    fn requirements(&self) -> BackingMode; // extented or scattered
    /// creates a resource managed by the gpu
    fn create_resource(
        &mut self,
        desc: &ResourceDesc,
        backing: &Backing,
    ) -> Result<ResourceHandle, DisplayError>; // may just validate + remember
    /// destroys the resource with handle `h`
    fn destroy_resource(&mut self, h: ResourceHandle) -> Result<(), DisplayError>;

    /// Writes the resource backing to the display
    fn scanout(&mut self, h: ResourceHandle) -> Result<(), DisplayError>;
    /// pushes buffered draw commands into the gpu
    fn flush(&mut self, h: ResourceHandle, rect: Option<ScreenRect>) -> Result<(), DisplayError>;
    /// waits for a vertical blank
    fn wait(&mut self) -> Result<(), DisplayError>;
    /// manages cursor display if the feature is available
    fn set_cursor(&mut self, x: u32, y: u32) -> Result<(), DisplayError>; // may return Unsupported
}
