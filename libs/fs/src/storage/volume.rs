use core::slice::GetDisjointMutError::IndexOutOfBounds;

use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
    vec::Vec,
};
use block::device::BlockDevice;

use crate::{
    collections::{bitmap::FSBitmap, cache::Cache, error::FSError, pathcache::FSPathCache},
    core::{
        locator::FSLocator,
        path::{self, FSPath},
    },
    meta::{
        context::{CreationContext, HeaderMetadata},
        header::FSHeader,
        inode::FSInode,
        permission::Permissions,
        superblock::Superblock,
    },
    storage::{
        blockstore::FSBlockStore,
        fileobject::{DirContents, FileObject, OpenMode},
        sector::{FSHeaderSector, FSInodeSector},
        volumeio::VolumeIO,
    },
};

const INODES_PER_SECTOR: u64 = 2;

/// Disk volume
pub struct FSVolume<'a, C: Cache = FSPathCache> {
    /// Address of the root header in this volume
    root: FSHeaderSector,
    /// Cache FSPath -> sector address
    cache: C,
    capacity: u64,
    io: VolumeIO<'a>,
}

impl<'a, C> FSVolume<'a, C>
where
    C: Cache,
{
    pub fn new(capacity: u64, io: VolumeIO<'a>) -> Self {
        Self {
            root: FSHeaderSector::new(0),
            cache: C::with_capacity(capacity as usize),
            capacity,
            io,
        }
    }
    /// Creates a new volume from a block device and formats it, fill cache, etc.
    pub fn from_block_device(blk_dev: &'a mut dyn BlockDevice) -> Result<Self, FSError> {
        let io = VolumeIO::from_block_device(blk_dev)?;
        let volume = Self::new(1000, io);
        Ok(volume)
    }
    /// rebuilds the path cache from disk by recursively walking the directory tree from the root
    fn populate_map(&mut self) -> Result<(), FSError> {
        self.populate_map_at(FSPath::new("/"), self.root)
    }
    /// recursivley cache path to sector from the root node
    fn populate_map_at(&mut self, path: &FSPath, sector: FSHeaderSector) -> Result<(), FSError> {
        self.cache.put_path(path, sector);

        let header = self.io.read_header_address(sector)?;
        let Some(inode_sector) = header.inode_addr() else {
            return Ok(());
        };
        if !header.is_dir() {
            return Ok(());
        }

        let inode = self.io.read_inode_address(inode_sector)?;
        let children: Vec<(String, FSHeaderSector)> = self
            .io
            .children_inode(header, sector, inode)?
            .map(|located| {
                (
                    // TODO: error handling on name
                    located.value().name().unwrap().to_string(),
                    FSHeaderSector::new(located.addr()),
                )
            })
            .collect();

        for (name, child_sector) in children {
            let child_path = if path.is_root() {
                format!("/{name}")
            } else {
                format!("{}/{name}", path.as_ref())
            };
            self.populate_map_at(FSPath::new(&child_path), child_sector)?;
        }
        Ok(())
    }
    /// Gets the physical sector of an `FSHeader` from a path
    fn resolve_path(&mut self, path: &FSPath) -> Result<FSHeaderSector, FSError> {
        if let Some(addr) = self.cache.get_path(path) {
            return Ok(addr);
        }
        // traverse root
        let mut components = path.components().peekable();
        let mut current_sector = self.root;
        let mut current_header = self.io.read_header_address(current_sector)?;

        while let Some(component) = components.next() {
            let inode_addr = current_header
                .inode_addr()
                .ok_or(FSError::Type("Expected directory".to_string()))?;

            // loop through the file data to find matching header
            let found = {
                let inode = self.io.read_inode_address(inode_addr)?;
                let mut found = None;
                for child in self
                    .io
                    .children_inode(current_header, current_sector, inode)?
                {
                    if child.value().name()?.eq(component) {
                        found = Some(child.addr())
                    }
                }
                found
            };
            // error if no match
            let address = found.ok_or_else(|| FSError::FileNotFound(path.as_ref().to_owned()))?;
            current_sector = FSHeaderSector::new(address);

            self.cache.put_path(path, current_sector);

            if components.peek().is_none() {
                return Ok(current_sector);
            }
            current_header = self.io.read_header_address(current_sector)?;
        }
        Err(FSError::file_not_found(String::new()))
    }

    pub fn create_file(
        &mut self,
        path: &FSPath,
        context: CreationContext,
        permissions: Permissions,
    ) -> Result<FSHeaderSector, FSError> {
        let parent_metadata = self.read_header(path.parent())?.metadata();
        let metadata = HeaderMetadata::new(
            context.uid,
            permissions.without_umask(context.umask),
            0,
            parent_metadata.storage_type,
        );
        let header = FSHeader::new_file(path.name(), metadata, context.now)?;
        self.create_header(path, header)
    }
    pub fn create_dir(
        &mut self,
        path: &FSPath,
        context: CreationContext,
        permissions: Permissions,
    ) -> Result<FSHeaderSector, FSError> {
        let parent_metadata = self.read_header(path.parent())?.metadata();
        let metadata = HeaderMetadata::new(
            context.uid,
            permissions.without_umask(context.umask),
            0,
            parent_metadata.storage_type,
        );
        let header = FSHeader::new_dir(path.name(), metadata, context.now)?;
        self.create_header(path, header)
    }
    pub fn create_header(
        &mut self,
        path: &FSPath,
        header: FSHeader,
    ) -> Result<FSHeaderSector, FSError> {
        let mut fo = self.open_read(path)?;
        fo.write_header(header)
            .map(|header_sector| {
                self.cache.put_path(path, header_sector);
                header_sector
            })
            .ok_or(FSError::IOError("Write failed"))
    }
    pub fn delete_file(&mut self, path: &FSPath) -> Result<FSHeader, FSError> {
        let sector = self.resolve_path(path)?;
        let header = self.io.delete_at(sector)?;
        self.cache.invalidate(path);
        Ok(header)
    }
    /// Writes a header to the next free header address. Returns the header address on success
    pub fn read_header(&mut self, path: &FSPath) -> Result<FSHeader, FSError> {
        let addr = self.resolve_path(path)?;
        self.io.read_header_address(addr)
    }

    pub fn read_inode(&mut self, path: &FSPath) -> Result<FSInode, FSError> {
        let header = self.read_header(path)?;
        if let Some(inode_addr) = header.inode_addr() {
            return self.io.read_inode_address(inode_addr);
        }
        Err(FSError::IOError("Failed to read inode"))
    }

    /* File Object Getters */
    // Gets a file object from the file located at `path`.
    pub fn open_read<'v>(&'v mut self, path: &FSPath) -> Result<FileObject<'v, 'a>, FSError> {
        let (header, header_sector, inode) = self.get_open_context(path)?;
        self.io
            .get_fileobj_inode(header, header_sector, inode, OpenMode::Read)
    }

    pub fn open_write<'v>(&'v mut self, path: &FSPath) -> Result<FileObject<'v, 'a>, FSError> {
        let (header, header_sector, inode) = self.get_open_context(path)?;
        self.io
            .get_fileobj_inode(header, header_sector, inode, OpenMode::Write)
    }

    pub fn open_append<'v>(&'v mut self, path: &FSPath) -> Result<FileObject<'v, 'a>, FSError> {
        let (header, header_sector, inode) = self.get_open_context(path)?;
        self.io
            .get_fileobj_inode(header, header_sector, inode, OpenMode::Append)
    }

    fn get_open_context(
        &mut self,
        path: &FSPath,
    ) -> Result<(FSHeader, FSHeaderSector, FSInode), FSError> {
        let header_sector = self.resolve_path(path)?;
        let header = self.io.read_header_address(header_sector)?;
        let inode = header
            .inode_addr()
            .ok_or(FSError::IOError("Failed to open file"))
            .map(|inode_address| self.io.read_inode_address(inode_address))?;
        Ok((header, header_sector, inode?))
    }

    /* Directory Children Getters */
    pub fn children<'v>(&'v mut self, path: &FSPath) -> Result<DirContents<'v, 'a>, FSError> {
        if self.is_file(path) {
            return Err(FSError::Type(format!(
                "Expected directory, {path:?} is a file"
            )));
        }
        self.open_read(path).map(|fobj| fobj.dir_contents())
    }

    /* path helper methods */
    pub fn is_file(&mut self, path: &FSPath) -> bool {
        self.read_header(path).is_ok_and(|h| h.is_dir())
    }
    pub fn is_dir(&mut self, path: &FSPath) -> bool {
        self.read_header(path).is_ok_and(|h| h.is_file())
    }
    pub fn exists(&mut self, path: &'a FSPath) -> bool {
        self.read_header(path).is_ok()
    }
}
impl<'a> Volume<'a> for FSVolume<'a> {
    fn resolve_path(&mut self, path: &FSPath) -> Result<FSHeaderSector, FSError> {
        self.resolve_path(path)
    }

    fn get_fileobj<'v>(&'v mut self, path: &FSPath) -> Result<FileObject<'v, 'a>, FSError> {
        self.open_read(path)
    }

    fn children<'v>(&'v mut self, path: &FSPath) -> Result<DirContents<'v, 'a>, FSError> {
        self.children(path)
    }

    fn read_inode(&mut self, path: &FSPath) -> Result<FSInode, FSError> {
        self.read_inode(path)
    }

    fn read_header(&mut self, path: &FSPath) -> Result<FSHeader, FSError> {
        self.read_header(path)
    }

    fn delete_file(&mut self, path: &FSPath) -> Result<FSHeader, FSError> {
        self.delete_file(path)
    }
}
trait Volume<'a> {
    fn resolve_path(&mut self, path: &FSPath) -> Result<FSHeaderSector, FSError>;
    fn get_fileobj<'v>(&'v mut self, path: &FSPath) -> Result<FileObject<'v, 'a>, FSError>;
    fn children<'v>(&'v mut self, path: &FSPath) -> Result<DirContents<'v, 'a>, FSError>;
    fn read_inode(&mut self, path: &FSPath) -> Result<FSInode, FSError>;
    fn read_header(&mut self, path: &FSPath) -> Result<FSHeader, FSError>;
    fn delete_file(&mut self, path: &FSPath) -> Result<FSHeader, FSError>;
}
