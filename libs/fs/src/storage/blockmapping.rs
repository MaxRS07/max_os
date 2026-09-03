use block::device::BlockDevice;

use crate::storage::sector::SectorAddr;

pub trait BlockMapping {
    fn get_phys(&self, blk_dev: &dyn BlockDevice) -> Option<SectorAddr> {
        None
    }
}
