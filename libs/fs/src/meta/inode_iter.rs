use crate::{meta::inode::FSInode, storage::blockstore::FSBlockStore};

/// Iterates logical addrs sequentially
pub struct InodeIter<'a> {
    inode: FSInode,
    index: u64,
    blk_store: &'a mut FSBlockStore<'a>,
}

impl<'a> InodeIter<'a> {
    pub fn new(inode: FSInode, blk_store: &'a mut FSBlockStore<'a>) -> Self {
        InodeIter {
            inode,
            index: 0,
            blk_store,
        }
    }
}
impl<'a> Iterator for InodeIter<'a> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        let addr = self.inode.map_logical(self.index, self.blk_store).ok()?;
        self.index += 1;
        if addr == 0 { None } else { Some(addr) }
    }
}
