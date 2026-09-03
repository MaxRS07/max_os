use core::{
    cmp::max,
    ptr::null,
    sync::atomic::{AtomicU8, AtomicU32, Ordering},
};

use alloc::{format, sync::Arc, vec::Vec};

use crate::{
    collections::{bitmap::FSBitmap, error::FSError, filelock::LockStateData},
    core::{mutpath::FSMutPath, path::FSPath},
    meta::{
        header::FSHeader,
        inode::{BLOCK_SIZE, FSInode},
        lockstate::LockState,
    },
    storage::{
        allocator::SectorAllocator,
        blockstore::FSBlockStore,
        sector::{self, FSHeaderSector, FSInodeSector},
    },
};

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

    /// Tries to read `buf.len()` bytes into the file buffer starting from the current cursor position. Returns the length of the read data advancing the cursor by `buffer.len()` bytes, `FSError` if the read failed
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, FSError> {
        let mut idx = 0;
        while idx < buf.len() {
            let Some(addr) = self.get_physical_sector() else {
                break;
            };
            let len = self.blk_store.read_to_buffer(addr, buf, idx)?;
            idx += len;
            self.cursor += 1;
        }
        Ok(idx)
    }
    /// Reads the next header inside this directory. If `Self` is not a directory, returns err
    pub fn read_next_header(&mut self) -> Option<Located<FSHeader>> {
        if self.align_to(align_of::<FSHeader>() as u64).is_err() {
            return None;
        }
        let header_sector = self.get_header_sector()?;
        let header = self.cread().ok()?;
        let located = Located::new(header_sector.get(), header);
        Some(located)
    }
    /// Shifts `self.cursor` to the next `align_of::<T>()` and reads the next `size_of::<T>()` bytes as `T`.
    fn cread<T: Copy>(&mut self) -> Result<T, FSError> {
        self.lock_state.lock_shared();
        let align = align_of::<T>() as u64;
        self.align_to(align)?;
        let val = self.blk_store.read(self.get_sector(), self.get_offset());
        self.lock_state.release_shared();
        val
    }

    /* Write methods */
    /// Writes bytes to the file at the cursor
    pub fn write_buffer(&mut self, buffer: &[u8]) -> Result<(), FSError> {
        self.cwrite_buffer(buffer)
    }
    /// Writes a header to this file object if it is a directory
    pub fn write_header(&mut self, header: FSHeader) -> Option<FSHeaderSector> {
        if !self.is_dir() {
            return None;
        }
        self.seek_free_header()?;
        if self.cwrite(header).is_ok() {
            self.get_header_sector()
        } else {
            None
        }
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
        self.align_to(align)?;
        self.blk_store
            .write(self.get_sector(), self.get_offset(), value)?;
        self.lock_state.release_exclusive();
        Ok(())
    }
    /// Writes a buffer at the cursor without alignment
    fn cwrite_buffer(&mut self, buffer: &[u8]) -> Result<(), FSError> {
        self.lock_state.lock_exclusive();
        self.check_size(buffer.len() as u64);
        let sector = self.get_sector();
        let offset = self.get_offset();
        self.blk_store.write_buffer(sector, offset, buffer)?;
        self.cursor += buffer.len() as u64;
        self.lock_state.release_exclusive();
        Ok(())
    }

    fn check_permission(&self, minimum: OpenMode) -> Result<(), FSError> {
        if self.mode < minimum {
            return Err(FSError::PermissionDenied {
                permission: OpenMode::from(self.mode).to_str(),
                needs: minimum.to_str(),
            });
        }
        Ok(())
    }

    /// Moves the cursor to the address of the next FSHeader
    fn seek_next_header(&mut self) -> Option<()> {
        if !self.is_dir() {
            return None;
        }
        let header_align = align_of::<FSHeader>() as u64;
        self.cursor += 1;
        if self.align_to(header_align).is_ok() {
            return Some(());
        }
        None
    }
    /// writes the asscosiated inode instance to disk
    fn sync_inode(&mut self) -> Result<(), FSError> {
        self.lock_state.lock_exclusive();
        let inode_address = FSInodeSector::new(self.inode.id());
        self.blk_store.write_inode(inode_address, self.inode)?;
        self.lock_state.release_exclusive();
        Ok(())
    }
    /* seek */
    /// Moves the cursor to the start of the next free header sector greater than or equal to `self.cursor`. Returns `Some(logical_byte)` if a free header was found. Returns `None` otherwise.    
    fn seek_free_header(&mut self) -> Option<()> {
        while self.seek_next_header().is_some() {
            if let Ok(header) = self.cread::<FSHeader>()
                && header.is_removed()
            {
                return Some(());
            }
        }
        None
    }
    /// moves the cursor to address of the next header matching `predicate`. Returns none if no matching header was found
    fn find_header<F>(&mut self, predicate: F) -> Option<()>
    where
        F: Fn(&FSHeader) -> bool,
    {
        while let Some(header) = self.read_next_header() {
            if predicate(header.value()) {
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
            return Err(FSError::OutOfBounds("Align exceeds file size"));
        }
        self.cursor = next;
        Ok(())
    }
    /// Allocates space if needed to store `bytes` extra bytes in the file. `Ok` if allocation was successful or none was required. `Err` if allocation fails.
    fn check_size(&mut self, bytes: u64) -> Result<(), FSError> {
        let bytes_needed = (self.cursor + bytes).saturating_sub(self.max_cursor());
        // no allocation is needed
        if bytes_needed == 0 {
            return Ok(());
        }
        let sectors_needed = bytes_needed.div_ceil(BLOCK_SIZE);
        self.inode
            .alloc_size(self.data_map, self.blk_store, sectors_needed)
            .ok_or(FSError::VolumeFull(
                "Failed to allocate bytes, volume is full",
            ))?;
        self.sync_inode()
    }
    /// returns the current logical sector of the cursor
    fn get_sector(&self) -> u64 {
        self.cursor / 4096
    }
    fn get_physical_sector(&mut self) -> Option<u64> {
        self.inode.map_logical(self.get_sector(), self.blk_store)
    }
    /// returns the cursor's position within its current sector. Returned value will be smaller than the byte size of a sector
    fn get_offset(&self) -> u64 {
        self.cursor % 4096
    }
    fn get_header_sector(&mut self) -> Option<FSHeaderSector> {
        let sector = self.get_physical_sector()?;
        let offset = self.get_offset();
        Some(FSHeaderSector::from_sector_offset(sector, offset))
    }
    fn max_cursor(&self) -> u64 {
        self.inode.size() * 4096
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
