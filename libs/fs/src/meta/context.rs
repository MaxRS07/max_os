use crate::meta::{inode::StorageType, permission::Permissions};

pub struct CreationContext {
    pub uid: u32,
    pub gid: u32,
    pub umask: Permissions,
    pub now: u64,
}

pub struct HeaderMetadata {
    pub owner: u32,
    pub permissions: Permissions,
    pub flags: u16,
    pub storage_type: StorageType,
}

impl HeaderMetadata {
    pub fn new(
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
