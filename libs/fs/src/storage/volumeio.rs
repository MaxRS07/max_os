use alloc::{format, string::String, vec::Vec};
use block::device::BlockDevice;

use crate::{
    collections::{bitmap::FSBitmap, error::FSError, filelock::LockTable},
    meta::{header::FSHeader, inode::FSInode, superblock::Superblock},
    storage::{
        allocator::SectorAllocator,
        blockstore::FSBlockStore,
        fileobject::{DirContents, FileObject, OpenMode},
        sector::{FSHeaderSector, FSInodeSector},
    },
};

pub struct VolumeIO<'a, Allocator = FSBitmap>
where
    Allocator: SectorAllocator,
{
    data_map: Allocator,
    inode_map: Allocator,
    blk_store: FSBlockStore<'a>,
    lock_table: LockTable,
}

const INODES_PER_SECTOR: u64 = 4;

impl<'a> VolumeIO<'a> {
    fn new(
        start_sector: u64,
        size: u64,
        inode_capacity: u64,
        blk_store: FSBlockStore<'a>,
    ) -> Result<Self, FSError> {
        let mut new = Self {
            data_map: FSBitmap::new(0, 0),
            inode_map: FSBitmap::new(0, 0),
            lock_table: LockTable::default(),
            blk_store,
        };
        new.set_maps(start_sector, size, inode_capacity)?;
        Ok(new)
    }
    /// Creates a VolumeIO object from a block device and formats it using the superblock if present.
    pub fn from_block_device(blk_dev: &'a mut dyn BlockDevice) -> Result<Self, FSError> {
        let blk_store = FSBlockStore::new(blk_dev);
        let mut volume_io = Self::new(0, 0, 0, blk_store)?;
        volume_io.load_superblock()?;
        Ok(volume_io)
    }

    /// verifies superblock formatting, populating this volume's metadata
    pub fn load_superblock(&mut self) -> Result<Superblock, FSError> {
        let block: Superblock = self.blk_store.read(0, 0)?;
        if block.verify() {
            // populate stored data
            self.set_maps(0, block.capacity, block.inodes as u64);
            return Ok(block);
        }
        Err(FSError::Corrupted(
            "Verification failed, volume may be corrupted or formatted incorrectly",
        ))
    }
    /// rebuilds and sets own empty maps. `data_map`'s size specified `capacity` in sectors. `inodes` is capacity of individual inodes. Fails if inodes overflow capacity
    fn set_maps(&mut self, start: u64, capacity: u64, inodes: u64) -> Result<(), FSError> {
        // file data is chunked in 8
        let data_bits = capacity.div_ceil(8);

        // 4/sector
        let inode_sectors = inodes.div_ceil(INODES_PER_SECTOR);

        if inode_sectors >= capacity {
            return Err(FSError::OutOfBounds(
                "Inode sectors exceeds volume capacity",
            ));
        }

        let mut data_map = FSBitmap::new(start, data_bits);
        let inode_map = FSBitmap::new(start, inode_sectors);

        data_map.set_used_cont(start, inode_sectors)?;

        self.data_map = data_map;
        self.inode_map = inode_map;

        Ok(())
    }
    pub(crate) fn get_fileobj_inode<'v>(
        &'v mut self,
        header: FSHeader,
        header_sector: FSHeaderSector,
        inode: FSInode,
        mode: OpenMode,
    ) -> Result<FileObject<'v, 'a>, FSError> {
        let lock_state = self
            .lock_table
            .get_lock(inode.id())
            .ok_or(FSError::IOError("Failed to retrieve lockstate for inode"));
        Ok(FileObject::new(
            header,
            header_sector,
            inode,
            &mut self.data_map,
            lock_state?,
            &mut self.blk_store,
            mode,
        ))
    }

    /// recursive method to delete contents of a directory
    pub fn delete_at(&mut self, sector: FSHeaderSector) -> Result<FSHeader, FSError> {
        let mut header = self.read_header_address(sector)?;

        if header.is_dir()
            && let Some(inode_sector) = header.inode_addr()
        {
            let inode = self.read_inode_address(inode_sector)?;
            let children: Vec<FSHeaderSector> = self
                .children_inode(header, sector, inode)?
                .map(|located| FSHeaderSector::new(located.addr()))
                .collect();

            for child_sector in children {
                self.delete_at(child_sector)?;
            }
        }

        header.set_removed(true);
        self.write_header(sector, header)?;
        Ok(header)
    }

    pub(crate) fn children_inode<'v>(
        &'v mut self,
        header: FSHeader,
        header_sector: FSHeaderSector,
        inode: FSInode,
    ) -> Result<DirContents<'v, 'a>, FSError> {
        let contents = self
            .get_fileobj_inode(header, header_sector, inode, OpenMode::Read)?
            .dir_contents();
        Ok(contents)
    }

    pub(crate) fn read_inode_address(
        &mut self,
        address: FSInodeSector,
    ) -> Result<FSInode, FSError> {
        self.blk_store.read_inode(address)
    }

    pub(crate) fn free_inode(&mut self, address: FSInodeSector) -> Result<FSInode, FSError> {
        let inode = self.blk_store.read_inode(address)?;
        self.inode_map.set_free(address.get())?;
        Ok(inode)
    }

    pub(crate) fn write_header(
        &mut self,
        sector: FSHeaderSector,
        header: FSHeader,
    ) -> Result<(), FSError> {
        self.blk_store.write_header(sector, header)
    }

    /* Inode Methods */
    /// Writes an Inode to disk at the next free Inode sector, fails if Inode sectors are exhausted
    fn write_inode(&mut self, inode: FSInode) -> Result<FSInodeSector, FSError> {
        if let Some(address) = self.inode_map.next_free() {
            let inode_addr = FSInodeSector::new(address);
            self.blk_store.write_inode(inode_addr, inode)?;
            return Ok(inode_addr);
        }
        Err(FSError::VolumeFull("Write failed, volume is full"))
    }

    pub fn read_header_address(&mut self, address: FSHeaderSector) -> Result<FSHeader, FSError> {
        self.blk_store.read_header(address)
    }
}
