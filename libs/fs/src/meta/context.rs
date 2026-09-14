use crate::meta::{header::FSHeader, inode::StorageType, permission::Permissions};

pub struct CreationContext {
    pub uid: u32,
    pub gid: u32,
    pub umask: Permissions,
    pub now: u64,
}
impl CreationContext {
    pub fn new(uid: u32, gid: u32, umask: Permissions, now: u64) -> Self {
        Self {
            uid,
            gid,
            umask,
            now,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HeaderMetadata {
    pub owner: u32,
    pub permissions: Permissions,
    pub flags: u16,
    pub storage_type: StorageType,
}

impl HeaderMetadata {
    pub const fn root_dir(owner: u32) -> Self {
        Self::new(
            owner,
            Permissions::ROOT,
            FSHeader::IS_ROOT,
            StorageType::Extents,
        )
    }

    pub const fn new(
        owner: u32,
        permissions: Permissions,
        flags: u16,
        storage_type: StorageType,
    ) -> Self {
        HeaderMetadata {
            owner,
            permissions,
            flags,
            storage_type,
        }
    }
}
