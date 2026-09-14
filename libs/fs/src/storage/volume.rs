use core::{debug_assert_eq, ptr::addr_of, slice::GetDisjointMutError::IndexOutOfBounds, todo};

use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
    vec::Vec,
};
use block::device::BlockDevice;

use crate::{
    VERSION,
    collections::{cache::Cache, error::FSError, pathcache::FSPathCache},
    core::{locator::FSLocator, path::FSPath},
    meta::{
        context::{CreationContext, HeaderMetadata},
        header::FSHeader,
        inode::{
            BLOCK_SIZE, Extent, FSInode, SECTOR_SIZE, SECTORS_PER_BLOCK, StorageType,
            block_to_sector,
        },
        permission::Permissions,
        superblock::Superblock,
    },
    storage::{
        blockstore::FSBlockStore,
        fileobject::{DirContents, FileObject, OpenMode},
        format::FormatOptions,
        sector::{FSHeaderSector, FSInodeSector},
        volumeio::{INODES_PER_SECTOR, VolumeIO},
    },
};

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
    pub fn new(root: FSHeaderSector, capacity: u64, io: VolumeIO<'a>) -> Self {
        Self {
            root,
            cache: C::with_capacity(capacity as usize),
            capacity,
            io,
        }
    }
    /// Creates a new volume from a block device and formats it, fill cache, etc.
    pub fn from_block_device(blk_dev: &'a mut dyn BlockDevice) -> Result<Self, FSError> {
        let (root, io) = VolumeIO::from_block_device(blk_dev)?;
        let mut volume = Self::new(root, 1000, io);
        volume.populate_map()?;
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
                let header = located.value();
                (
                    // TODO: error handling on name
                    header.name().unwrap_or("name error").to_string(),
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
        // The root component identifies the volume root rather than a child entry.
        let mut current_sector = self.root;
        for component in path.components() {
            let current_header = self.io.read_header_address(current_sector)?;
            let inode_addr = current_header
                .inode_addr()
                .ok_or_else(|| FSError::Type("Expected directory".to_string()))?;
            let inode = self.io.read_inode_address(inode_addr)?;

            let children = self
                .io
                .children_inode(current_header, current_sector, inode)?;

            let found = self
                .io
                .children_inode(current_header, current_sector, inode)?
                .find(|child| child.value().name().is_ok_and(|name| name == component))
                .map(|child| child.addr());

            current_sector = FSHeaderSector::new(
                found.ok_or_else(|| FSError::FileNotFound(path.as_ref().to_owned()))?,
            );
        }

        self.cache.put_path(path, current_sector);
        Ok(current_sector)
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
        let mut header = FSHeader::new_file(path.name(), metadata, context.now)?;
        self.create_header(path, &mut header)
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
        let mut header = FSHeader::new_dir(path.name(), metadata, context.now)?;
        self.create_header(path, &mut header)
    }
    /// Writes `header` to the inode at `path.parent()`, placing it in the directory
    fn create_header(
        &mut self,
        path: &FSPath,
        header: &mut FSHeader,
    ) -> Result<FSHeaderSector, FSError> {
        self.allocate_header(header, header.metadata().storage_type)?;
        let mut fo = self.open_read(path.parent())?;
        fo.write_header(*header).inspect(|header_sector| {
            self.cache.put_path(path, *header_sector);
        })
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
    /// Allocates an Inode in the volume and attributes it to `header`. Inode will be preallocated with `size` bytes
    fn allocate_header(
        &mut self,
        header: &mut FSHeader,
        kind: StorageType,
    ) -> Result<FSInodeSector, FSError> {
        let inode_addr = self.io.allocate_inode(kind)?;
        header.set_inode_addr(inode_addr);
        Ok(inode_addr)
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
    /* Reformat on mount, write superblock and create root folder */
    pub fn format(blk_dev: &'a mut dyn BlockDevice, opts: FormatOptions) -> Result<(), FSError> {
        let mut blk_store = FSBlockStore::new(blk_dev);

        // write superblock to start of first sector
        let inode_sectors = (opts.inodes as u64).div_ceil(INODES_PER_SECTOR);
        let meta_sectors = 1 + inode_sectors;
        let data_start = meta_sectors.div_ceil(SECTORS_PER_BLOCK);
        let total_blocks = blk_store.capacity() / SECTORS_PER_BLOCK;
        total_blocks
            .checked_sub(data_start)
            .filter(|n| *n > 0)
            .ok_or(FSError::OutOfBounds(
                "Metadata exceeds the volume's capacity".to_owned(),
            ))?;

        // the inode table must start zeroed: a slot with a null id is what marks it free
        for sector in 1..=inode_sectors {
            blk_store.write_buffer(sector, 0, &[0u8; SECTOR_SIZE as usize])?;
        }

        // the root directory starts out owning a single block and grows through
        // `alloc_size` like any other directory
        let root_inode_addr = FSInodeSector::from_sector_offset(1, 0);
        let mut root_node = FSInode::empty_ext();
        root_node.set_extents(&[Extent {
            logical_block: 0,
            physical_block: data_start,
            block_count: 1,
        }]);
        if let FSInode::Extents { id, size, .. } = &mut root_node {
            *id = root_inode_addr.get();
            *size = 1;
        }
        blk_store.write_inode(root_inode_addr, root_node)?;

        let meta = HeaderMetadata::new(
            0,
            Permissions::ROOT,
            FSHeader::IS_ROOT,
            StorageType::Extents,
        );
        let mut root = FSHeader::new_dir("/", meta, 0)?;
        root.set_inode_addr(root_inode_addr);
        let root_sector = FSHeaderSector::from_sector_offset(0, 256);
        blk_store.write_header(root_sector, root)?;

        blk_store.write_buffer(block_to_sector(data_start), 0, &[0u8; BLOCK_SIZE as usize])?;

        let superblock = Superblock::new(VERSION, root_sector, opts.capacity, opts.inodes);

        blk_store.write(0, 0, superblock)?;
        Ok(())
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
    fn format(&mut self) -> Result<(), FSError> {
        todo!()
    }
}
trait Volume<'a> {
    fn resolve_path(&mut self, path: &FSPath) -> Result<FSHeaderSector, FSError>;
    fn get_fileobj<'v>(&'v mut self, path: &FSPath) -> Result<FileObject<'v, 'a>, FSError>;
    fn children<'v>(&'v mut self, path: &FSPath) -> Result<DirContents<'v, 'a>, FSError>;
    fn read_inode(&mut self, path: &FSPath) -> Result<FSInode, FSError>;
    fn read_header(&mut self, path: &FSPath) -> Result<FSHeader, FSError>;
    fn delete_file(&mut self, path: &FSPath) -> Result<FSHeader, FSError>;
    fn format(&mut self) -> Result<(), FSError>;
}
