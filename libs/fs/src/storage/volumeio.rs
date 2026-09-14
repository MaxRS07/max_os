use alloc::{format, string::String, vec::Vec};
use block::device::BlockDevice;

use crate::{
    collections::{bitmap::FSBitmap, error::FSError, filelock::LockTable},
    core::path::FSPath,
    meta::{
        header::FSHeader,
        inode::{FSInode, SECTOR_SIZE, SECTORS_PER_BLOCK, StorageType},
        superblock::Superblock,
    },
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

/// On-disk slot size of an `FSInode`.
pub(crate) const INODE_SLOT: u64 = 128;
/// Four 128-byte inodes share a 512 byte sector.
pub(crate) const INODES_PER_SECTOR: u64 = SECTOR_SIZE / INODE_SLOT;
/// Inode 0 is the volume root, written by `format`, and is never handed out by the allocator.
const ROOT_INODE_INDEX: u64 = 0;

impl<'a> VolumeIO<'a> {
    fn new(blk_store: FSBlockStore<'a>) -> Self {
        Self {
            data_map: FSBitmap::new(0, 0),
            inode_map: FSBitmap::new(0, 0),
            lock_table: LockTable::default(),
            blk_store,
        }
    }
    /// Creates a VolumeIO object from a block device and formats it using the superblock if present.
    pub fn from_block_device(
        blk_dev: &'a mut dyn BlockDevice,
    ) -> Result<(FSHeaderSector, Self), FSError> {
        let blk_store = FSBlockStore::new(blk_dev);
        let mut volume_io = Self::new(blk_store);
        let sb = volume_io.load_superblock()?;
        Ok((sb.root, volume_io))
    }

    /// verifies superblock formatting, populating this volume's metadata
    pub fn load_superblock(&mut self) -> Result<Superblock, FSError> {
        let block: Superblock = self.blk_store.read(0, 0)?;
        if block.verify() {
            // populate stored data
            self.set_maps(0, block.capacity, block.inodes as u64)?;
            self.rebuild_maps(block.inodes as u64)?;
            return Ok(block);
        }
        Err(FSError::Corrupted(
            "Verification failed, volume may be corrupted or formatted incorrectly",
        ))
    }
    /// rebuilds and sets own empty maps. `capacity` is the volume size in raw sectors, `inodes`
    /// the number of inode slots. Fails if the metadata region overflows the volume.
    ///
    /// `data_map` has one bit per 4KiB *block*, `inode_map` one bit per *inode slot*.
    fn set_maps(&mut self, start: u64, capacity: u64, inodes: u64) -> Result<(), FSError> {
        // file data is chunked into 8-sector blocks
        let data_blocks = capacity / SECTORS_PER_BLOCK;

        // superblock sector + the inode table, rounded up to a whole number of blocks
        let inode_sectors = inodes.div_ceil(INODES_PER_SECTOR);
        let meta_blocks = (1 + inode_sectors).div_ceil(SECTORS_PER_BLOCK);

        if meta_blocks >= data_blocks {
            return Err(FSError::OutOfBounds(format!(
                "Metadata blocks exceed volume capacity ({meta_blocks}/{data_blocks})"
            )));
        }

        let mut data_map = FSBitmap::new(start, data_blocks);
        let mut inode_map = FSBitmap::new(start, inodes);

        // the superblock and inode table live at the front of the volume
        data_map.set_used_cont(start, meta_blocks)?;
        // the root inode is written by `format` and must never be reallocated
        inode_map.set_used(start + ROOT_INODE_INDEX)?;

        self.data_map = data_map;
        self.inode_map = inode_map;

        Ok(())
    }
    /// Walks the on-disk inode table and marks every live inode slot, plus every block that
    /// inode owns, as used. Without this both maps would come up empty after a mount and the
    /// allocator would hand out blocks that already hold live data.
    fn rebuild_maps(&mut self, inodes: u64) -> Result<(), FSError> {
        for slot in 0..inodes {
            let inode = self.blk_store.read_inode(Self::inode_slot_addr(slot))?;
            // a zeroed slot has a null id, every allocated inode stores its own address
            if inode.id() == 0 {
                continue;
            }
            self.inode_map.set_used(slot)?;
            inode.mark_allocated(&mut self.data_map, &mut self.blk_store)?;
        }
        Ok(())
    }

    /// Address of inode `slot`. The inode table starts at sector 1, directly after the superblock.
    fn inode_slot_addr(slot: u64) -> FSInodeSector {
        FSInodeSector::from_sector_offset(
            1 + slot / INODES_PER_SECTOR,
            (slot % INODES_PER_SECTOR) * INODE_SLOT,
        )
    }

    /// Inverse of [`Self::inode_slot_addr`]
    fn inode_slot(address: FSInodeSector) -> u64 {
        (address.sector() - 1) * INODES_PER_SECTOR + address.offset() / INODE_SLOT
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
        let mut inode = self.blk_store.read_inode(address)?;
        inode.free_node(&mut self.data_map, self.blk_store.device());
        self.inode_map.set_free(Self::inode_slot(address))?;
        // clear the slot so `rebuild_maps` sees it as free on the next mount
        self.blk_store.write_inode(address, FSInode::empty_ind())?;
        Ok(inode)
    }
    pub(crate) fn allocate_inode(&mut self, kind: StorageType) -> Result<FSInodeSector, FSError> {
        let addr = self
            .inode_map
            .take_free()
            .map(Self::inode_slot_addr)
            .ok_or(FSError::VolumeFull("Inode table exhausted"))?;
        let mut inode = FSInode::empty(kind);
        inode.set_id(addr.get());
        self.blk_store.write_inode(addr, inode)?;
        Ok(addr)
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
