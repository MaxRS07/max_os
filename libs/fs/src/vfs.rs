use alloc::format;
// Handles file traversal and file mapping
use block::device::BlockDevice;

use crate::{
    collections::error::FSError,
    core::{locator::FSLocator, mount::FSMountTable, path::FSPath},
    meta::{context::CreationContext, header::FSHeader, permission::Permissions},
    storage::{fileobject::FileObject, sector::FSHeaderSector, volume::FSVolume},
};
pub struct Vfs<'a> {
    mount_table: FSMountTable<'a>,
}

impl<'a> Vfs<'a> {
    pub fn new(_blk_dev: &'static mut dyn BlockDevice) -> Self {
        Self {
            mount_table: FSMountTable::new(),
        }
    }
    /// mounts a volume to a path
    pub fn mount(
        &mut self,
        path: &'a FSPath,
        blk_dev: &'a mut dyn BlockDevice,
    ) -> Result<u32, FSError> {
        self.mount_table.mount(path, blk_dev)
    }
    /// Attempts to unmount the volume at `path`. Returns the removed volume if found and the volume is not the primary volume at `"/"`, otherwise `None`.
    pub fn unmount(&mut self, path: &'a FSPath) -> Option<FSVolume<'a>> {
        if path.is_root() {
            return None;
        }
        self.mount_table.unmount(path)
    }
    pub fn get_file<'v>(&'v mut self, path: &FSPath) -> Result<FileObject<'v, 'a>, FSError> {
        if let Some(vol) = self.resolve(path) {
            return vol.open_read(path);
        }
        Err(FSError::FileNotFound(format!(
            "Failed to resolve path {path:?}"
        )))
    }

    pub fn create_file(
        &mut self,
        path: &FSPath,
        context: CreationContext,
        permissions: Permissions,
    ) -> Result<FSHeaderSector, FSError> {
        if let Some(vol) = self.resolve(path) {
            return vol.create_file(path, context, permissions);
        }

        Err(FSError::Other)
    }

    /// Retuns the volume
    pub fn resolve(&mut self, path: &FSPath) -> Option<&mut FSVolume<'a>> {
        if !path.is_absolute() {
            return None;
        }
        self.mount_table.find_volume(path)
    }
}
