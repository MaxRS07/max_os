use collections::lrucache::LRUCache;

use crate::{
    collections::{cache::Cache, trie::FSRouteTrie},
    core::path::FSPath,
    meta::header::FSHeader,
    storage::sector::FSHeaderSector,
};
pub struct FSPathCache {
    /// Maps a trie address to
    lru: LRUCache<usize, FSHeaderSector>,
    trie: FSRouteTrie,
}

impl FSPathCache {
    pub fn new(capacity: usize) -> Self {
        FSPathCache {
            lru: LRUCache::new(capacity),
            trie: FSRouteTrie::new(),
        }
    }
    /// Caches an `FSPath` with a file header sector address, replacing existing values, returns `Some` if the the cache was successful, `None` otherwise.
    pub fn put_path(&mut self, path: &FSPath, sector: FSHeaderSector) -> Option<()> {
        if let Some(id) = self.trie.find(path) {
            self.lru.put(id, sector);
            return Some(());
        }
        if let Some(id) = self.trie.insert(path)
            && let Some(_) = self.lru.put(id, sector)
        {
            return Some(());
        }
        None
    }
    /// Caches a path with a file header sector adress, replacing existing values, returns `Some` if the the cache was successful, `None` otherwise.
    pub fn put_str(&mut self, path: &str, sector: FSHeaderSector) -> Option<()> {
        let fspath = FSPath::new(path);
        self.put_path(fspath, sector)
    }
    /// Retuns the address of `FileHeader` at `path`, or `None` if the path is not cached
    pub fn get_path(&mut self, path: &FSPath) -> Option<FSHeaderSector> {
        if let Some(id) = self.trie.find(path) {
            return self.lru.get(id);
        }
        None
    }
    /// Retuns the address of `FileHeader` at `path`, or `None` if the path is not cached
    pub fn get_str(&mut self, path: &str) -> Option<FSHeaderSector> {
        let fspath = FSPath::new(path);
        self.get_path(fspath)
    }
    /// Invalidates a path, removing it and references from the trie and lru.
    pub fn invalidate_path(&mut self, path: &FSPath) -> Option<FSHeaderSector> {
        let node_id = self.trie.find(path)?;
        self.trie.remove(node_id);
        self.lru.remove_node(&node_id)
    }
}

impl Cache for FSPathCache {
    fn with_capacity(capacity: usize) -> Self {
        Self::new(capacity)
    }
    fn get_path(&mut self, path: &FSPath) -> Option<FSHeaderSector> {
        self.get_path(path)
    }
    fn put_path(&mut self, path: &FSPath, sector: FSHeaderSector) -> Option<()> {
        self.put_path(path, sector)
    }
    fn invalidate(&mut self, path: &FSPath) -> Option<FSHeaderSector> {
        self.invalidate_path(path)
    }
}
