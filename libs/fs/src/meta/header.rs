use core::{
    cmp::Ordering,
    fmt::{Debug, Display, Write},
    todo,
};

use alloc::string::ToString;
use block::device::BlockDevice;

use crate::{
    collections::error::FSError,
    core::{mutpath::FSMutPath, path::FSPath},
    meta::{
        context::{self, CreationContext, HeaderMetadata},
        inode::{FSInode, StorageType},
        permission::Permissions,
    },
    storage::sector::FSInodeSector,
};

pub const MAX_NAME_LEN: usize = 124;

/// Represents a file system object on disk.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FSHeader {
    /// name of the file including extension. 256 char max
    name: [u8; MAX_NAME_LEN],
    name_len: u8,
    /// general metadata flags
    /// 15 header type
    /// 13 R0
    /// 12 resizable
    /// 11-1 unused
    /// 0 - is present on disk
    flags: u16,
    map_type: StorageType,
    /// payload size in bytes
    size: u64,
    /// date created
    created: u64,
    /// date modified
    modified: u64,

    /// user id of file owner
    owner: u32,
    /// unix permissions bitmask
    permissions: Permissions,
    /// innode
    inode_addr: FSInodeSector,
}

impl FSHeader {
    pub const IS_DIRECTORY: u16 = 1;
    pub const IS_RESIZABLE: u16 = 1 << 12;
    pub const IS_ROOT: u16 = 1 << 15;
    pub const IS_REMOVED: u16 = 1 << 14;

    /* PUBLIC GETTERS */
    pub fn name(&self) -> Result<&str, FSError> {
        if self.name_len == 0 {
            return Ok("");
        }
        let bytes = &self.name[0..self.name_len as usize];
        str::from_utf8(bytes)
            .map_err(|_| FSError::Corrupted("Name bytes corrupted, failed to read"))
    }
    pub fn inode_addr(&self) -> Option<FSInodeSector> {
        if self.active() {
            return Some(self.inode_addr);
        }
        None
    }
    pub fn set_inode_addr(&mut self, value: FSInodeSector) {
        self.inode_addr = value;
    }

    pub fn new_dir(name: &str, metadata: HeaderMetadata, now: u64) -> Result<Self, FSError> {
        let size = 0;
        let mut flags: u16 = metadata.flags;
        let type_mask = 0b11;
        flags |= Self::IS_DIRECTORY;
        flags |= flags & !type_mask;

        Self::new(
            name,
            size,
            now,
            now,
            metadata.owner,
            metadata.permissions,
            flags,
            metadata.storage_type,
        )
    }
    pub fn new_file(name: &str, metadata: HeaderMetadata, now: u64) -> Result<Self, FSError> {
        let flags = metadata.flags & !Self::IS_DIRECTORY;

        Self::new(
            name,
            0,
            now,
            now,
            metadata.owner,
            metadata.permissions,
            flags,
            metadata.storage_type,
        )
    }
    /// Creates a growable file with preallocated sectors. Faster for files with a known minimum size.
    pub fn sized_file(name: &str, metadata: HeaderMetadata, now: u64) -> Result<Self, FSError> {
        let flags: u16 = metadata.flags & !Self::IS_DIRECTORY;
        Self::new(
            name,
            0,
            now,
            now,
            metadata.owner,
            metadata.permissions,
            flags,
            metadata.storage_type,
        )
    }
    /// Creates a growable directory with preallocated sectors. Faster for directories with a known minimum size.
    pub fn sized_dir(name: &str, metadata: HeaderMetadata, now: u64) -> Result<Self, FSError> {
        let flags: u16 = metadata.flags | Self::IS_DIRECTORY;
        Self::new(
            name,
            0,
            now,
            now,
            metadata.owner,
            metadata.permissions,
            flags,
            metadata.storage_type,
        )
    }
    pub fn is_dir(&self) -> bool {
        self.flags & Self::IS_DIRECTORY != 0
    }
    pub fn is_file(&self) -> bool {
        !self.is_dir()
    }
    pub fn is_resizable(&self) -> bool {
        self.flags & Self::IS_RESIZABLE != 0
    }
    pub fn is_root_dir(&self) -> bool {
        self.is_dir() && self.flags & Self::IS_ROOT != 0
    }
    pub fn is_removed(&self) -> bool {
        self.flags & Self::IS_REMOVED != 0
    }
    pub fn set_removed(&mut self, value: bool) {
        self.flags &= !Self::IS_REMOVED;
        self.flags |= value as u16 * Self::IS_REMOVED
    }
    /// whether this file is on disk
    pub fn active(&self) -> bool {
        !self.inode_addr.is_null()
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    /// Returns this header's metadata used
    pub fn metadata(&self) -> HeaderMetadata {
        HeaderMetadata::new(self.owner, self.permissions, self.flags, self.map_type)
    }
    /// Private unchecked new
    #[allow(clippy::too_many_arguments)]
    fn new(
        name: &str,
        size: u64,
        created: u64,
        modified: u64,
        owner: u32,
        permissions: Permissions,
        flags: u16,
        storage_type: StorageType,
    ) -> Result<Self, FSError> {
        let name_bytes = Self::name_to_bytes(name)?;
        Ok(Self {
            name: name_bytes,
            name_len: name.len() as u8,
            size,
            created,
            modified,
            owner,
            inode_addr: FSInodeSector::new(0),
            permissions,
            flags,
            map_type: storage_type,
        })
    }
    fn name_to_bytes(name: &str) -> Result<[u8; MAX_NAME_LEN], FSError> {
        if name.len() > MAX_NAME_LEN {
            return Err(FSError::IOError(
                "FSHeader name len exceeds max name length",
            ));
        }
        let mut name_bytes = [0; MAX_NAME_LEN];
        name_bytes[0..name.len()].copy_from_slice(name.as_bytes());
        Ok(name_bytes)
    }
}
/// Name based equality for default alphabetical sort
impl PartialEq for FSHeader {
    fn eq(&self, other: &Self) -> bool {
        self.name()
            .is_ok_and(|name| other.name().is_ok_and(|other| name == other))
    }
}
impl PartialOrd for FSHeader {
    fn ge(&self, other: &Self) -> bool {
        self.gt(other) || self.eq(other)
    }
    fn le(&self, other: &Self) -> bool {
        !self.gt(other) || self.eq(other)
    }
    fn gt(&self, other: &Self) -> bool {
        self.name()
            .is_ok_and(|name| other.name().is_ok_and(|other| name >= other))
    }
    fn lt(&self, other: &Self) -> bool {
        !self.gt(other) && self.ne(other)
    }
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(if self.lt(other) {
            Ordering::Less
        } else if self.gt(other) {
            Ordering::Greater
        } else {
            Ordering::Equal
        })
    }
}
/// Guarentee header compiles to 256 bytes
const _: () = {
    assert!(core::mem::size_of::<FSHeader>() < 256);
};

impl Display for FSHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name_str = unsafe { str::from_utf8_unchecked(&self.name[..self.name_len as usize]) };
        let _ = f.write_str("FSHeader { ");
        let _ = f.write_fmt(format_args!("name: {}, ", name_str));
        let _ = f.write_fmt(format_args!("name_len: {}, ", self.name_len));
        let _ = f.write_fmt(format_args!("active: {}, ", self.active()));
        let _ = f.write_fmt(format_args!("created: {}, ", self.created));
        let _ = f.write_fmt(format_args!("modified: {:b}, ", self.modified));
        let _ = f.write_fmt(format_args!("flags: {:b}, ", self.flags));
        let _ = f.write_fmt(format_args!("inode: {}, ", self.inode_addr));
        let _ = f.write_fmt(format_args!("map: {:?} ", self.map_type));
        f.write_str("}")
    }
}
