use crate::{core::path::FSPath, storage::sector::FSHeaderSector};

/// Trait for file system path -> FSHeaderSector caches
pub trait Cache {
    fn with_capacity(capacity: usize) -> Self;
    fn put_path(&mut self, path: &FSPath, sector: FSHeaderSector) -> Option<()>;
    fn get_path(&mut self, path: &FSPath) -> Option<FSHeaderSector>;
    fn invalidate(&mut self, path: &FSPath) -> Option<FSHeaderSector>;
}
