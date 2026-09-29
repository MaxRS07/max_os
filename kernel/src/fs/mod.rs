use core::cell::OnceCell;

use block::device::BlockDevice;
use fs::{
    collections::{bitmap::FSBitmap, error::FSError, pathcache::FSPathCache},
    core::path::FSPath,
    meta::{context::CreationContext, permission::Permissions},
    storage::{fileobject::OpenMode, format::FormatOptions, volume::FSVolume},
    vfs::Vfs,
};
use log::{Level::Debug, debug, warn};
use sync::{mutex::Mutex, oncelock::OnceLock};

pub mod call;

pub static GLOBAL_FS: OnceLock<Mutex<Vfs<'static>>> = OnceLock::new();

pub fn fs_init(blk_dev: &'static mut dyn BlockDevice, remount: bool) -> Result<(), FSError> {
    let mut vfs: Vfs = Vfs::new();
    let capacity = blk_dev.capacity();
    if remount {
        FSVolume::<FSPathCache>::format(
            blk_dev,
            FormatOptions {
                version: 0,
                inodes: 100,
                capacity,
            },
        )?;
    }
    let bytes = blk_dev.capacity() * blk_dev.sector_size() as u64;
    debug!("Mounting primary drive, ({bytes} bytes)");
    vfs.mount(FSPath::new("/"), blk_dev)?;
    debug!("Primary drive mounted");

    GLOBAL_FS.set(Mutex::new(vfs));
    Ok(())
}
