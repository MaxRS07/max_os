use core::{
    cmp::max,
    fmt::Display,
    ptr::null,
    sync::atomic::{AtomicU8, AtomicU32, Ordering},
};

use alloc::{borrow::ToOwned, format, sync::Arc, vec::Vec};

use crate::{
    collections::{bitmap::FSBitmap, error::FSError, filelock::LockStateData},
    core::{mutpath::FSMutPath, path::FSPath},
    meta::{
        header::FSHeader,
        inode::{BLOCK_SIZE, BlockPos, FSInode},
        lockstate::LockState,
    },
    storage::{
        allocator::SectorAllocator,
        blockstore::FSBlockStore,
        sector::{self, FSHeaderSector, FSInodeSector},
    },
};

/// On-disk slot size of an `FSHeader`. Keeps a header from straddling a sector boundary.
const HEADER_SLOT: u64 = 256;

trait FileIO {
    fn is_dir();
    fn write_buffer();
    /* Directory only methods */
    fn read_next_header();
    /// Writes a new header into the directory
    fn write_header();
    /// Removes a header from directory
    fn remove_header(&mut self);
    /// Tries to fill `buffer` with bytes from this file object, returning the write length if successful
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, FSError>;

    /* File methods */
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OpenMode {
    Read,
    Write,
    Append,
}
impl OpenMode {
    pub fn to_str(&self) -> &'static str {
        match self {
            OpenMode::Append => "Append",
            OpenMode::Write => "Write",
            OpenMode::Read => "Read",
        }
    }
}
impl From<u8> for OpenMode {
    fn from(value: u8) -> Self {
        match value {
            1 => OpenMode::Write,
            2 => OpenMode::Append,
            _ => OpenMode::Read,
        }
    }
}
/// Open file object in RAM
pub struct FileObject<'v, 'a> {
    header: FSHeader,
    header_sector: FSHeaderSector,
    inode: FSInode,
    /// ref of file lock state, shared between FileObjects with the same inode
    lock_state: Arc<LockStateData>,
    data_map: &'v mut dyn SectorAllocator,
    /// current position (logical byte)
    cursor: u64,
    blk_store: &'v mut FSBlockStore<'a>,
    mode: OpenMode,
}

impl<'v, 'a> FileObject<'v, 'a> {
    pub fn new(
        header: FSHeader,
        header_sector: FSHeaderSector,
        inode: FSInode,
        data_map: &'v mut dyn SectorAllocator,
        lock_state: Arc<LockStateData>,
        blk_store: &'v mut FSBlockStore<'a>,
        mode: OpenMode,
    ) -> Self {
        let cursor = match mode {
            OpenMode::Append => header.size(),
            _ => 0,
        };
        Self {
            header,
            header_sector,
            inode,
            data_map,
            cursor,
            lock_state,
            blk_store,
            mode,
        }
    }

    /* Flag get/set */
    pub fn is_dir(&self) -> bool {
        self.header.is_dir()
    }

    /* Read methods */

    /// Tries to read `buf.len()` bytes from the current cursor position, stopping at the end of
    /// the inode's allocated data. Returns the length of the read data, advancing the cursor by it
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, FSError> {
        let limit = self.max_cursor();
        let mut idx = 0;
        while idx < buf.len() && self.cursor < limit {
            let pos = self.position()?;
            let len = self.transfer_len(&pos, buf.len() - idx, limit);

            self.blk_store
                .read_at(pos.sector, pos.sector_offset, &mut buf[idx..idx + len])?;

            idx += len;
            self.cursor += len as u64;
        }
        Ok(idx)
    }
    /// Reads the next header inside this directory. If `Self` is not a directory, returns err
    pub fn read_next_header(&mut self) -> Option<Located<FSHeader>> {
        if self.align_to(HEADER_SLOT).is_err() {
            return None;
        }
        let Ok(header_sector) = self.get_header_sector() else {
            return None;
        };
        let header: FSHeader = self.cread().ok()?;
        // an unwritten slot terminates the directory
        if !header.active() {
            return None;
        }
        let located = Located::new(header_sector.get(), header);
        self.advance(HEADER_SLOT);
        Some(located)
    }
    /// Shifts `self.cursor` to the next `align_of::<T>()` and reads the next `size_of::<T>()` bytes as `T`.
    fn cread<T: Copy>(&mut self) -> Result<T, FSError> {
        self.lock_state.lock_shared();
        let align = align_of::<T>() as u64;
        let val = self.align_to(align).and_then(|_| {
            let pos = self.position()?;
            self.blk_store.read(pos.sector, pos.sector_offset)
        });
        self.lock_state.release_shared();
        val
    }

    /* Write methods */
    /// Writes bytes to the file at the cursor
    pub fn write_buffer(&mut self, buffer: &[u8]) -> Result<(), FSError> {
        self.cwrite_buffer(buffer)
    }
    /// Writes a header to this file object if it is a directory
    pub fn write_header(&mut self, header: FSHeader) -> Result<FSHeaderSector, FSError> {
        if !self.is_dir() {
            return Err(FSError::Type(format!(
                "Expected directory, {} is a file",
                header.name().unwrap_or("READ_ERROR")
            )));
        }
        self.seek_free_header();
        self.cwrite(header)?;
        self.get_header_sector()
    }
    pub fn remove_header(&mut self, name: &str) -> Result<(), FSError> {
        // move cursor to first header with matching name
        self.find_header(|header| header.name().is_ok_and(|header_name| header_name == name));
        let mut header = self.cread::<FSHeader>()?;
        header.set_removed(true);
        self.cwrite(header)
    }
    /// Performs a write at the cursor. Fails if the write overflows, align doesnt match, etc
    fn cwrite<T>(&mut self, value: T) -> Result<(), FSError> {
        self.lock_state.lock_exclusive();
        let align = align_of::<T>() as u64;
        let res = self
            .check_size(size_of::<T>() as u64)
            .and(self.align_to(align))
            .and_then(|_| {
                let pos = self.position()?;
                self.blk_store.write(pos.sector, pos.sector_offset, value)
            });
        self.lock_state.release_exclusive();
        res
    }
    /// Writes a buffer at the cursor without alignment
    fn cwrite_buffer(&mut self, buffer: &[u8]) -> Result<(), FSError> {
        self.lock_state.lock_exclusive();
        let res = self.write_buffer_positioned(buffer);
        self.lock_state.release_exclusive();
        res
    }
    /// Body of `cwrite_buffer`, split out so `?` cannot return while the lock is held
    fn write_buffer_positioned(&mut self, buffer: &[u8]) -> Result<(), FSError> {
        self.check_size(buffer.len() as u64)?;
        let limit = self.max_cursor();

        let mut idx = 0;
        while idx < buffer.len() {
            let pos = self.position()?;
            let len = self.transfer_len(&pos, buffer.len() - idx, limit);
            if len == 0 {
                return Err(FSError::OutOfBounds(
                    "Write exceeds the blocks allocated to this file".to_owned(),
                ));
            }

            self.blk_store
                .write_at(pos.sector, pos.sector_offset, &buffer[idx..idx + len])?;

            idx += len;
            self.cursor += len as u64;
        }
        Ok(())
    }

    fn check_permission(&self, minimum: OpenMode) -> Result<(), FSError> {
        if self.mode < minimum {
            return Err(FSError::PermissionDenied {
                permission: self.mode.to_str(),
                needs: minimum.to_str(),
            });
        }
        Ok(())
    }

    /// writes the asscosiated inode instance to disk
    fn sync_inode(&mut self) -> Result<(), FSError> {
        let inode_address = FSInodeSector::new(self.inode.id());
        self.blk_store.write_inode(inode_address, self.inode)
    }
    /* seek */
    /// Moves the cursor to the start of the next free header sector greater than or equal to `self.cursor`. Returns `Some(logical_byte)` if a free header was found. Returns `None` otherwise.    
    fn seek_free_header(&mut self) -> Option<()> {
        if !self.is_dir() {
            return None;
        }
        loop {
            self.align_to(HEADER_SLOT).ok()?;
            let header = self.cread::<FSHeader>().ok()?;
            if !header.active() || header.is_removed() {
                return Some(());
            }
            self.advance(HEADER_SLOT)?;
        }
    }
    /// moves the cursor to address of the next header matching `predicate`. Returns none if no matching header was found
    fn find_header<F>(&mut self, predicate: F) -> Option<()>
    where
        F: Fn(&FSHeader) -> bool,
    {
        while let Some(header) = self.read_next_header() {
            if predicate(header.value()) {
                self.cursor -= HEADER_SLOT;
                return Some(());
            }
        }
        None
    }
    /* cursor helpers */

    /// aligns the cursor to `align`. Fails if the next align overflows `self.max_cursor()`
    pub fn align_to(&mut self, align: u64) -> Result<(), FSError> {
        let next = self.cursor.next_multiple_of(align);
        if next > self.max_cursor() {
            return Err(FSError::OutOfBounds("Align exceeds file size".to_owned()));
        }
        self.cursor = next;
        Ok(())
    }
    // moves the cursor `bytes` bytes forward. Returns None if the the advance is greater than the size of the file
    fn advance(&mut self, bytes: u64) -> Option<()> {
        if self.cursor + bytes > self.max_cursor() {
            return None;
        }
        self.cursor += bytes;
        Some(())
    }
    fn check_size(&mut self, bytes: u64) -> Result<(), FSError> {
        let end = self.cursor + bytes;
        let bytes_needed = end.saturating_sub(self.max_cursor());
        if bytes_needed == 0 {
            return Ok(());
        }
        let first_new_block = self.inode.size();
        let blocks_needed = bytes_needed.div_ceil(BLOCK_SIZE);
        self.inode
            .alloc_size(self.data_map, self.blk_store, blocks_needed)
            .ok_or(FSError::VolumeFull(
                "Failed to allocate bytes, volume is full",
            ))?;
        for block in first_new_block..self.inode.size() {
            let physical = self.inode.map_logical(block, self.blk_store)?;
            self.blk_store.zero_block(physical)?;
        }
        self.sync_inode()?;

        Ok(())
    }
    /// Resolves the cursor against the inode's block map
    fn position(&mut self) -> Result<BlockPos, FSError> {
        self.inode.map_byte(self.cursor, self.blk_store)
    }
    /// Largest transfer that may start at `pos`, capped by `wanted`, the end of `pos`'s block,
    /// and `limit`
    fn transfer_len(&self, pos: &BlockPos, wanted: usize, limit: u64) -> usize {
        (wanted as u64)
            .min(pos.block_remaining)
            .min(limit.saturating_sub(self.cursor)) as usize
    }
    fn get_header_sector(&mut self) -> Result<FSHeaderSector, FSError> {
        let pos = self.position()?;
        Ok(FSHeaderSector::from_sector_offset(
            pos.sector,
            pos.sector_offset,
        ))
    }
    fn max_cursor(&self) -> u64 {
        self.inode.size() * BLOCK_SIZE
    }
    pub fn dir_contents(self) -> DirContents<'v, 'a> {
        DirContents::from_fileobj(self)
    }
}

/// Iterator over file headers in a directory
pub struct DirContents<'v, 'a> {
    file_obj: FileObject<'v, 'a>,
}
impl<'v, 'a> DirContents<'v, 'a> {
    pub fn from_fileobj(file_obj: FileObject<'v, 'a>) -> Self {
        Self { file_obj }
    }
}
impl<'v, 'a> Iterator for DirContents<'v, 'a> {
    type Item = Located<FSHeader>;

    fn next(&mut self) -> Option<Self::Item> {
        let header = self.file_obj.read_next_header()?;
        Some(header)
    }
}

pub struct Located<T> {
    addr: u64,
    value: T,
}
impl<T> Located<T> {
    pub fn new(addr: u64, value: T) -> Self {
        Self { addr, value }
    }
    pub fn addr(&self) -> u64 {
        self.addr
    }
    pub fn value(&self) -> &T {
        &self.value
    }
}
