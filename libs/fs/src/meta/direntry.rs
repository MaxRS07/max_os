#[repr(C)]
#[derive(Clone, Debug)]
pub struct DirEntry {
    inode: usize,
    entry_len: usize,
    name_len: usize,
    file_type: u8,
    name: [u8; 255],
}
