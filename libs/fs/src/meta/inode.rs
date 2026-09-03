use core::cmp::min;

use block::device::BlockDevice;

use crate::{
    collections::bitmap::FSBitmap,
    meta::inode_iter::InodeIter,
    storage::{allocator::SectorAllocator, blockstore::FSBlockStore, sector::SectorAddr},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StorageType {
    Indirect,
    Extents,
}

pub const DIRECT_LEN: usize = 6;
const NUM_EXTENTS: usize = 3;

/// Raw device sector size in bytes.
pub const SECTOR_SIZE: u64 = 512;
/// Logical allocation unit used by inode pointers: 8 raw sectors (4096 bytes).
pub const BLOCK_SIZE: u64 = 4096;
/// Raw sectors per logical block.
pub const SECTORS_PER_BLOCK: u64 = BLOCK_SIZE / SECTOR_SIZE;
/// Number of `SectorAddr` entries that fit in a single indirect block.
pub const ADDRS_PER_BLOCK: u64 = BLOCK_SIZE / size_of::<SectorAddr>() as u64;

/// Converts a logical block number (as stored in an inode's direct/indirect
/// pointers) into the raw device sector it begins at.
pub(crate) const fn block_to_sector(block: u64) -> u64 {
    block * SECTORS_PER_BLOCK
}

/// Unix-type inode for describing files contigously or fragmened blocks of memory. Data blocks are stored in 8-sector chunks, 4096 bytes each.
#[repr(C, u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FSInode {
    /// Storage by indirect memory using doubles, triples with nested sector addresses. Best for fragmented disk or small files
    Indirect {
        /// unique inode number
        id: u64,
        /// number of occupied data blocks (4KiB each). Secondary and tertiary index blocks are not counted.
        size: u64,
        /// block pointers 0..5 stored directly, each addressing a 4KiB block. will always be fully used if indirect is active
        direct: [u64; 6],
        /// primary index block containing `ADDRS_PER_BLOCK` pointers to data blocks.
        primary: u64,
        /// secondary index block containing pointers to primary blocks
        secondary: u64,
        /// tertiary index block containing pointers to secondary blocks, for very large files
        triple: u64,
    },
    /// Storage using multiple large chunks of contiguous memory. Best for desscribing large files.
    Extents {
        /// unique inode number
        id: u64,
        size: u64,
        extents: [Extent; 3],
        extent_count: u16,
    },
}

/// Maximum number of 4KiB data blocks addressable via indirect pointers.
pub const MAX_INDIRECT_LEN: u64 =
    DIRECT_LEN as u64 + ADDRS_PER_BLOCK + ADDRS_PER_BLOCK.pow(2) + ADDRS_PER_BLOCK.pow(3);
/// Maximum file size in bytes addressable via indirect pointers.
pub const MAX_INODE_SIZE: u64 = MAX_INDIRECT_LEN * BLOCK_SIZE;

impl FSInode {
    pub fn empty(kind: StorageType) -> Self {
        match kind {
            StorageType::Indirect => Self::empty_ind(),
            _ => Self::empty_ext(),
        }
    }

    pub fn empty_ind() -> Self {
        FSInode::Indirect {
            id: 0,
            size: 0,
            direct: [0; DIRECT_LEN],
            primary: 0,
            secondary: 0,
            triple: 0,
        }
    }
    pub fn empty_ext() -> Self {
        FSInode::Extents {
            id: 0,
            size: 0,
            extent_count: 0,
            extents: [Extent::empty(); 3],
        }
    }
    pub fn id(&self) -> u64 {
        match self {
            FSInode::Extents { id, .. } | FSInode::Indirect { id, .. } => *id,
        }
    }
    /// Returns the number of sectors owned by this `FSInode`. This value is also the greatest logical sector
    pub fn size(&self) -> u64 {
        match self {
            FSInode::Extents { size, .. } | FSInode::Indirect { size, .. } => *size,
        }
    }
    /// Allocates `len` more physical sectors on disk for this `Inode`. Returns `Some(())` and increments `self.size` if the allocation was successful, `None` otherwise
    pub fn alloc_size(
        &mut self,
        map: &mut dyn SectorAllocator,
        block_store: &mut FSBlockStore,
        sectors: u64,
    ) -> Option<()> {
        match self {
            FSInode::Extents { size, extents, .. } => {
                let mut remaining = sectors;
                let mut regions = [(0, 0); NUM_EXTENTS];
                for (i, ext) in extents.iter_mut().enumerate() {
                    if remaining == 0 {
                        return None;
                    }
                    let (start, l) = map.largest_cont(remaining);
                    // Not enough contiguous space for extents, fallback on indirect
                    if i == 0 && l < sectors / 4 {
                        return None;
                    }
                    regions[i] = (start, l);
                    *ext = Extent {
                        logical_block: 0,
                        physical_block: start,
                        block_count: l,
                        flags: 0,
                    };
                    remaining -= l;
                }
                if remaining > 0 {
                    return None;
                }
                regions.map(|(s, l)| map.set_used_cont(s, l));
                *size += sectors;
                Some(())
            }
            FSInode::Indirect {
                size: len,
                direct,
                primary,
                secondary,
                triple,
                ..
            } => {
                if *len + sectors > MAX_INDIRECT_LEN {
                    return None;
                }

                let max_direct = direct.len() as u64;
                let direct_len = min(max_direct, sectors);

                // Number of addresses that fit in a single 4KiB block
                let addrs_per_block = ADDRS_PER_BLOCK;

                let max_primary = addrs_per_block;
                let primary_len = min(max_primary, sectors.saturating_sub(max_direct));

                let max_secondary = addrs_per_block * addrs_per_block;
                let secondary_len = min(
                    max_secondary,
                    sectors
                        .saturating_sub(max_direct)
                        .saturating_sub(primary_len),
                );

                let tertiary_len = min(
                    max_secondary,
                    sectors
                        .saturating_sub(max_direct)
                        .saturating_sub(primary_len)
                        .saturating_sub(secondary_len),
                );

                // direct blocks
                if direct_len > 0 {
                    let mut buffer = alloc::vec![0; direct_len as usize];
                    map.take_free_buf(&mut buffer);
                    direct[..direct_len as usize].copy_from_slice(&buffer[..direct_len as usize]);
                }

                // primary block
                if primary_len > 0 {
                    let primary_block_sector = map.next_free()?;
                    *primary = primary_block_sector;

                    let mut addr_buf = alloc::vec![0; primary_len as usize];
                    map.take_free_buf(&mut addr_buf)?;
                    Self::pack_primary(block_store, primary_block_sector, &addr_buf)?;
                }

                // double
                if secondary_len > 0 {
                    let secondary_block_sector = map.next_free()?;
                    *secondary = secondary_block_sector;

                    let mut addr_buf = alloc::vec![0; secondary_len as usize];
                    map.take_free_buf(&mut addr_buf)?;
                    Self::pack_secondary(map, block_store, secondary_block_sector, &addr_buf)?;
                }
                if tertiary_len > 0 {
                    let tertiary_block_sector = map.next_free()?;
                    *triple = tertiary_block_sector;

                    let mut addr_buf = alloc::vec![0; tertiary_len as usize];
                    map.take_free_buf(&mut addr_buf)?;
                    Self::pack_tertiary(map, block_store, tertiary_block_sector, &addr_buf)?;
                }
                *len += sectors;
                Some(())
            }
        }
    }
    /// frees all sectors allocated to this `Inode`. Returns `None` if the operation failed
    pub fn free_node(&mut self, map: &mut FSBitmap, blk_dev: &mut dyn BlockDevice) -> Option<()> {
        match self {
            FSInode::Extents { extents, .. } => {
                for extent in extents {
                    map.set_free_cont(extent.physical_block, extent.block_count);
                    extent.clear();
                }
                Some(())
            }
            FSInode::Indirect {
                size: len,
                direct,
                primary,
                secondary,
                triple,
                ..
            } => {
                if *len == 0 {
                    return Some(());
                }
                let addrs_per_block = ADDRS_PER_BLOCK as usize;
                let max_direct = direct.len() as u64;
                let mut remaining = *len;

                // 1. Free Direct Blocks
                let direct_len = min(max_direct, remaining);
                direct.iter_mut().for_each(|dir| {
                    if *dir != 0 {
                        map.set_free(*dir);
                        *dir = 0;
                    }
                });
                remaining -= direct_len;

                // free primary
                if remaining > 0 && *primary != 0 {
                    let primary_len = min(addrs_per_block as u64, remaining);
                    let mut buf = alloc::vec![0u64; addrs_per_block];
                    if blk_dev
                        .read_buffer(block_to_sector(*primary), as_bytes_mut(&mut buf))
                        .is_err()
                    {
                        return None;
                    }

                    for &sector in &buf[..primary_len as usize] {
                        if sector != 0 {
                            map.set_free(sector);
                        }
                    }
                    map.set_free(*primary);
                    *primary = 0;
                    remaining -= primary_len;
                }

                // free secondary
                if remaining > 0 && *secondary != 0 {
                    let max_secondary = addrs_per_block as u64 * addrs_per_block as u64;
                    let secondary_len = min(max_secondary, remaining);
                    let num_primaries = secondary_len.div_ceil(addrs_per_block as u64);

                    let mut sec_buf = alloc::vec![0u64; addrs_per_block];
                    if blk_dev
                        .read_buffer(block_to_sector(*secondary), as_bytes_mut(&mut sec_buf))
                        .is_err()
                    {
                        return None;
                    }

                    for (i, sector) in sec_buf.iter().take(num_primaries as usize).enumerate() {
                        if *sector == 0 {
                            continue;
                        }

                        let chunk_len = min(
                            addrs_per_block as u64,
                            secondary_len - (i as u64) * addrs_per_block as u64,
                        );
                        let mut prim_buf = alloc::vec![0u64; addrs_per_block];
                        if blk_dev
                            .read_buffer(block_to_sector(*sector), as_bytes_mut(&mut prim_buf))
                            .is_err()
                        {
                            return None;
                        }

                        for &sector in &prim_buf[..chunk_len as usize] {
                            if sector != 0 {
                                map.set_free(sector);
                            }
                        }
                        map.set_free(*sector);
                    }
                    map.set_free(*secondary);
                    *secondary = 0;
                    remaining -= secondary_len;
                }

                // free tert
                if remaining > 0 && *triple != 0 {
                    let sec_capacity = addrs_per_block as u64 * addrs_per_block as u64;
                    let max_tertiary = sec_capacity * addrs_per_block as u64;
                    let tertiary_len = min(max_tertiary, remaining);
                    let num_secondaries = tertiary_len.div_ceil(sec_capacity);

                    let mut tert_buf = alloc::vec![0u64; addrs_per_block];
                    if blk_dev
                        .read_buffer(block_to_sector(*triple), as_bytes_mut(&mut tert_buf))
                        .is_err()
                    {
                        return None;
                    }

                    for (i, sec_ptr) in tert_buf.iter().take(num_secondaries as usize).enumerate() {
                        if *sec_ptr == 0 {
                            continue;
                        }

                        // Blocks accounted for by this secondary block
                        let sec_len = min(sec_capacity, tertiary_len - (i as u64) * sec_capacity);
                        let num_primaries = sec_len.div_ceil(addrs_per_block as u64);

                        let mut sec_buf = alloc::vec![0u64; addrs_per_block];
                        if blk_dev
                            .read_buffer(block_to_sector(*sec_ptr), as_bytes_mut(&mut sec_buf))
                            .is_err()
                        {
                            return None;
                        }

                        for (j, prim_ptr) in sec_buf.iter().take(num_primaries as usize).enumerate()
                        {
                            if *prim_ptr == 0 {
                                continue;
                            }

                            let chunk_len = min(
                                addrs_per_block as u64,
                                sec_len - (j as u64) * addrs_per_block as u64,
                            );
                            let mut prim_buf = alloc::vec![0u64; addrs_per_block];
                            if blk_dev
                                .read_buffer(
                                    block_to_sector(*prim_ptr),
                                    as_bytes_mut(&mut prim_buf),
                                )
                                .is_err()
                            {
                                return None;
                            }

                            for &data_block in &prim_buf[..chunk_len as usize] {
                                if data_block != 0 {
                                    map.set_free(data_block);
                                }
                            }
                            map.set_free(*prim_ptr);
                        }
                        map.set_free(*sec_ptr);
                    }
                    map.set_free(*triple);
                    *triple = 0;
                }
                *len = 0;
                Some(())
            }
        }
    }
    /// Maps a logical sector within an inode to a physical sector address
    pub fn map_logical(&self, logical: u64, blk_store: &mut FSBlockStore<'_>) -> Option<u64> {
        match self {
            FSInode::Extents { size, extents, .. } => {
                if logical >= *size {
                    return None;
                }
                let mut cur = 0u64;
                for ext in extents {
                    if logical < cur + ext.block_count {
                        let addr = ext.physical_block + (logical - cur);
                        return Some(addr);
                    }
                    cur += ext.block_count;
                }
                None
            }
            FSInode::Indirect {
                size,
                direct,
                primary,
                secondary,
                triple,
                ..
            } => {
                if logical >= *size {
                    return None;
                }

                let direct_end = DIRECT_LEN as u64;
                let primary_end = direct_end + ADDRS_PER_BLOCK;
                let sec_capacity = ADDRS_PER_BLOCK * ADDRS_PER_BLOCK;
                let secondary_end = primary_end + sec_capacity;

                if logical < direct_end {
                    return Some(direct[logical as usize]);
                }
                if logical < primary_end {
                    let local = (logical - direct_end) as usize;
                    return read_block_entry(blk_store, *primary, local);
                }
                if logical < secondary_end {
                    let local = logical - primary_end;
                    let prim_logical = (local / ADDRS_PER_BLOCK) as usize;
                    let prim_local = (local % ADDRS_PER_BLOCK) as usize;

                    let prim_block = read_block_entry(blk_store, *secondary, prim_logical)?;
                    return read_block_entry(blk_store, prim_block, prim_local);
                }

                let local = logical - secondary_end;
                let sec_logical = (local / sec_capacity) as usize;
                let rem = local % sec_capacity;
                let prim_logical = (rem / ADDRS_PER_BLOCK) as usize;
                let prim_local = (rem % ADDRS_PER_BLOCK) as usize;

                let sec_block = read_block_entry(blk_store, *triple, sec_logical)?;
                let prim_block = read_block_entry(blk_store, sec_block, prim_logical)?;
                read_block_entry(blk_store, prim_block, prim_local)
            }
        }
    }

    pub fn into_iter<'a>(self, blk_store: &'a mut FSBlockStore<'a>) -> InodeIter<'a> {
        InodeIter::new(self, blk_store)
    }

    /// Writes block addresses into a single primary (indirect) block on disk.
    fn pack_primary(blk: &mut FSBlockStore, block: SectorAddr, addrs: &[SectorAddr]) -> Option<()> {
        let mut primary_addrs = alloc::vec![0; ADDRS_PER_BLOCK as usize];
        primary_addrs[..addrs.len()].copy_from_slice(addrs);

        let primary_bytes = as_bytes(&primary_addrs);
        if blk
            .write_buffer(block_to_sector(block), 0, primary_bytes)
            .is_err()
        {
            return None;
        }
        Some(())
    }

    /// Creates a secondary (double indirect) block pointing to primary blocks,
    /// each containing data block addresses.
    fn pack_secondary(
        map: &mut dyn SectorAllocator,
        blk: &mut FSBlockStore,
        sec_block: SectorAddr,
        addrs: &[SectorAddr],
    ) -> Option<()> {
        let addrs_per_block = ADDRS_PER_BLOCK as usize;
        let num_primary_blocks = addrs.len().div_ceil(addrs_per_block);
        let mut primary_blocks = alloc::vec![0; num_primary_blocks];

        for (i, block) in primary_blocks.iter_mut().enumerate() {
            let prim_block = map.next_free()?;
            *block = prim_block;

            let start = i * addrs_per_block;
            let end = min(addrs.len(), start + addrs_per_block);
            Self::pack_primary(blk, prim_block, &addrs[start..end]);
        }

        // Write the secondary index block containing pointers to all allocated primary blocks
        Self::pack_primary(blk, sec_block, &primary_blocks);
        Some(())
    }

    /// Creates a tertiary (triple indirect) block pointing to secondary blocks.
    fn pack_tertiary(
        map: &mut dyn SectorAllocator,
        blk: &mut FSBlockStore,
        tert_block: SectorAddr,
        addrs: &[SectorAddr],
    ) -> Option<()> {
        let addrs_per_block = ADDRS_PER_BLOCK as usize;
        let sec_capacity = addrs_per_block * addrs_per_block;
        let num_secondary_blocks = addrs.len().div_ceil(sec_capacity);
        let mut secondary_blocks = alloc::vec![0; num_secondary_blocks];

        for (i, block) in secondary_blocks.iter_mut().enumerate() {
            let sec_block = map.next_free()?;
            *block = sec_block;

            let start = i * sec_capacity;
            let end = min(addrs.len(), start + sec_capacity);
            Self::pack_secondary(map, blk, sec_block, &addrs[start..end])?;
        }

        Self::pack_primary(blk, tert_block, &secondary_blocks);
        Some(())
    }
}

/// Reads a single pointer entry out of an indirect block on disk
fn read_block_entry(blk_store: &mut FSBlockStore, block: u64, index: usize) -> Option<u64> {
    let mut buf = alloc::vec![0u64; ADDRS_PER_BLOCK as usize];
    blk_store.read_index_block(block, &mut buf).ok()?;
    buf.get(index).copied()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Extent {
    pub logical_block: SectorAddr,
    pub physical_block: SectorAddr,
    pub block_count: u64,
    pub flags: u16,
}

impl Extent {
    // TODO: add flag docs and layout
    pub const fn empty() -> Self {
        Extent {
            logical_block: 0,
            physical_block: 0,
            block_count: 0,
            flags: 0,
        }
    }
    pub fn clear(&mut self) {
        self.block_count = 0;
        self.flags = 0;
        self.logical_block = 0;
        self.physical_block = 0;
    }
    /// True if the Inode contains at least one sector
    pub fn is_used(&self) -> bool {
        self.block_count != 0
    }
}

const _: () = assert!(size_of::<FSInode>() == 128);

pub fn as_bytes<T: Copy>(slice: &[T]) -> &[u8] {
    let byte_len = core::mem::size_of_val(slice);
    unsafe { core::slice::from_raw_parts(slice.as_ptr() as *const u8, byte_len) }
}

pub fn as_bytes_mut<T: Copy>(slice: &mut [T]) -> &mut [u8] {
    let byte_len = core::mem::size_of_val(slice);
    unsafe { core::slice::from_raw_parts_mut(slice.as_ptr() as *mut u8, byte_len) }
}
