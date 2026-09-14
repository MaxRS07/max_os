use block::device::BlockDevice;
use fs::{
    collections::{bitmap::FSBitmap, error::FSError, pathcache::FSPathCache},
    core::path::FSPath,
    meta::{context::CreationContext, permission::Permissions},
    storage::{fileobject::OpenMode, format::FormatOptions, volume::FSVolume},
    vfs::Vfs,
};
use log::{Level::Debug, debug};

use crate::console::writer::println;

pub mod call;

pub fn fs_init(blk_dev: &mut (dyn BlockDevice + 'static), remount: bool) -> Result<(), FSError> {
    let mut vfs = Vfs::new();
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

    let permissions = Permissions::from_raw(0);
    let context = CreationContext::new(0, 0, permissions, 0);
    let path = FSPath::new("/file.txt");
    vfs.create_file(path, context, permissions)?;
    debug!("created file.txt");

    let mut fo = vfs.open(path, OpenMode::Write)?;
    let buffer = b"Welcome to my file";
    debug!("Opened root");
    fo.write_buffer(buffer)?;
    debug!("Wrote bytes");

    let mut fo = vfs.open(FSPath::new("/file.txt"), OpenMode::Read)?;
    let mut read_buf = [0u8; 20];
    fo.read(&mut read_buf)?;
    let text = str::from_utf8(&read_buf).unwrap_or("READ ERROR");
    debug!("{text}");
    // vfs.get_file(path)?.read(&mut buf)?;
    Ok(())
}
