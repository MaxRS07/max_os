use alloc::format;
// Handles file traversal and file mapping
use block::device::BlockDevice;

use crate::{
    collections::error::FSError,
    core::{locator::FSLocator, mount::FSMountTable, path::FSPath},
    meta::{context::CreationContext, header::FSHeader, permission::Permissions},
    storage::{
        fileobject::{FileObject, OpenMode},
        sector::FSHeaderSector,
        volume::FSVolume,
    },
};
pub struct Vfs<'a> {
    mount_table: FSMountTable<'a>,
}

impl<'a> Vfs<'a> {
    pub fn new() -> Self {
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
    pub fn open<'v>(
        &'v mut self,
        path: &FSPath,
        mode: OpenMode,
    ) -> Result<FileObject<'v, 'a>, FSError> {
        if let Some(vol) = self.resolve(path) {
            return match mode {
                OpenMode::Append => vol.open_append(path),
                OpenMode::Read => vol.open_read(path),
                OpenMode::Write => vol.open_write(path),
            };
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
        Err(FSError::Resolve(format!(
            "Failed to resolve path: \"{path}\""
        )))
    }

    /// Retuns the volume
    pub fn resolve(&mut self, path: &FSPath) -> Option<&mut FSVolume<'a>> {
        if !path.is_absolute() {
            return None;
        }
        self.mount_table.find_volume(path)
    }
}

impl<'a> Default for Vfs<'a> {
    fn default() -> Self {
        Self::new()
    }
}
