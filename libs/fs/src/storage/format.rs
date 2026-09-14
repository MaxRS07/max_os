// Format types and utils
#[derive(Clone, Copy, Debug)]
pub struct FormatOptions {
    pub version: u32,
    /// Inode capacity
    pub inodes: u32,
    pub capacity: u64,
}
