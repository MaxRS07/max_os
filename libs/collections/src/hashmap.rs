const BUCKET_COUNT: usize = 64;

use core::{
    alloc::Layout,
    fmt::Debug,
    hash::{BuildHasher, Hash, Hasher},
    option::Iter,
    ptr::null_mut,
    sync::atomic::{Ordering, fence},
};

use alloc::alloc;
use log::info;

struct HashNode<K, V> {
    key: K,
    value: V,
    next: *mut HashNode<K, V>,
    prev: *mut HashNode<K, V>,
}
pub struct BucketIterator<K, V> {
    next: *mut HashNode<K, V>,
}
impl<K, V: Clone> Iterator for BucketIterator<K, V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.next.is_null() {
            let current = unsafe { &*self.next };
            self.next = current.next;
            return Some(current.value.clone());
        }
        None
    }
}
pub struct HashMap<K: Hash + Eq + Clone, V: Clone, S: BuildHasher = FnvBuildHasher> {
    hasher: S,
    len: usize,
    buckets: [*mut HashNode<K, V>; BUCKET_COUNT],
    current_bucket: usize,
    current_node: *mut HashNode<K, V>,
}

impl<K, V> HashMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            hasher: FnvBuildHasher,
            len: 0,
            buckets: [null_mut(); BUCKET_COUNT],
            current_bucket: 0,
            current_node: null_mut(),
        }
    }
}

impl<K, V, S> HashMap<K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: BuildHasher,
{
    pub fn empty() -> Self
    where
        S: Default,
    {
        Self {
            hasher: S::default(),
            len: 0,
            buckets: [null_mut(); BUCKET_COUNT],
            current_bucket: 0,
            current_node: null_mut(),
        }
    }
    /// gets a value from the map, returning it's value or `None` if the key is not present
    pub fn get(&self, key: &K) -> Option<V> {
        let idx = (self.hasher.hash_one(key) % BUCKET_COUNT as u64) as usize;
        let mut current_ptr = self.buckets[idx];
        while !current_ptr.is_null() {
            unsafe {
                if (*current_ptr).key.eq(key) {
                    return Some((*current_ptr).value.clone());
                }
                current_ptr = (*current_ptr).next
            }
        }
        None
    }
    pub fn get_iter(&self, key: K) -> BucketIterator<K, V> {
        let idx = self.hasher.hash_one(&key) as usize % BUCKET_COUNT;
        let current_ptr = self.buckets[idx];
        BucketIterator { next: current_ptr }
    }
    pub fn get_iter_idx(&self, bucket: usize) -> BucketIterator<K, V> {
        if bucket > BUCKET_COUNT {
            panic!("index out of range")
        }
        let head = self.buckets[bucket];
        BucketIterator { next: head }
    }
    /// Inserts a value into the map. If the key already exists, the value is updated and `Some(old_value)` is returned. Returns `None` otherwise.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let idx = self.hasher.hash_one(&key) as usize % BUCKET_COUNT;
        let head = self.buckets[idx];
        let mut current_ptr = self.buckets[idx];

        // update value on key collision
        unsafe {
            while !current_ptr.is_null() {
                if (*current_ptr).key.eq(&key) {
                    let old_v = (*current_ptr).value.clone();
                    (*current_ptr).value = value;
                    return Some(old_v);
                }
                current_ptr = (*current_ptr).next;
            }
        }
        unsafe {
            let new_node_start = alloc::alloc(Self::node_layout());
            let new_node_ptr = new_node_start as *mut HashNode<K, V>;
            fence(Ordering::SeqCst);
            if new_node_start.is_null() {
                panic!("GlobalAllocater allocated null ptr")
            }
            let new_node: HashNode<K, V> = HashNode {
                key,
                value,
                next: head,
                prev: null_mut(),
            };
            // use write to prevent memory casting bugs, safety where K, V can't drop
            core::ptr::write(new_node_ptr, new_node);

            // current should point back to new when the bucket already has nodes
            if !current_ptr.is_null() {
                (*current_ptr).prev = new_node_ptr;
            }
            self.len += 1;
            // set new to bucket head
            self.buckets[idx] = new_node_ptr;
        }

        None
    }
    /// Removes a entry from the map. Returns `Some(value)` asscosiated with `key` if `key` is present, otherwise `None`
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let idx = self.hasher.hash_one(&key) as usize % BUCKET_COUNT;
        let mut current_ptr = self.buckets[idx];
        unsafe {
            while !current_ptr.is_null() {
                if (*current_ptr).key.eq(&key) {
                    // next.prev should point back to prev, skipping over current
                    if !(*current_ptr).next.is_null() {
                        (*(*current_ptr).next).prev = (*current_ptr).prev;
                    }
                    // prev.next should point to next, skipping over current
                    if !(*current_ptr).prev.is_null() {
                        (*(*current_ptr).prev).next = (*current_ptr).next;
                    } else {
                        self.buckets[idx] = (*current_ptr).next
                    }

                    return Some((*current_ptr).value.clone());
                }
                current_ptr = (*current_ptr).next
            }
        }
        None
    }
    // removes all entries from the map
    pub fn clear(&mut self) {
        let layout = Self::node_layout();
        for i in 0..self.buckets.len() {
            let mut head = self.buckets[i];
            while !head.is_null() {
                let start_pointer = head as *mut u8;
                unsafe {
                    let next_ptr = (*head).next;
                    alloc::dealloc(start_pointer, layout);
                    head = next_ptr
                };
            }
            self.buckets[i] = null_mut();
        }
        self.len = 0;
        self.current_bucket = 0;
        self.current_node = null_mut();
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn node_layout() -> Layout {
        let size = size_of::<HashNode<K, V>>();
        let align = align_of::<HashNode<K, V>>();
        Layout::from_size_align(size, align).unwrap()
    }
}

impl<K: Debug + Hash + Eq + Clone, V: Debug + Clone> Debug for HashMap<K, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let _ = f.write_fmt(format_args!("HashMap {{ len: {}, ", self.len));
        if !self.is_empty() {
            let _ = f.write_str("entries: {\n");
            let iter = self.clone();
            for (k, v) in iter {
                let _ = f.write_fmt(format_args!("  (key: {:?}, value: {:?})\n", k, v));
            }
        }
        let _ = f.write_str("}");
        Ok(())
    }
}
impl<K: Hash + Eq + Clone, V: Clone> Iterator for HashMap<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_node.is_null() {
            if self.current_bucket >= BUCKET_COUNT {
                return None; // We've exhausted every bucket.
            }
            // Move to the next bucket head
            self.current_node = self.buckets[self.current_bucket];
            self.current_bucket += 1;
        }

        unsafe {
            let node_ptr = self.current_node;

            self.current_node = (*node_ptr).next;

            let key = core::ptr::read(&(*node_ptr).key);
            let value = core::ptr::read(&(*node_ptr).value);

            alloc::dealloc(node_ptr as *mut u8, Self::node_layout());

            Some((key, value))
        }
    }
}

impl<K, V, S> Default for HashMap<K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: BuildHasher + Default,
{
    fn default() -> Self {
        Self::empty()
    }
}
impl<K, V> Clone for HashMap<K, V>
where
    K: core::hash::Hash + Eq + Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        let mut new_map = HashMap::new();

        for i in 0..BUCKET_COUNT {
            let mut current_node = self.buckets[i];

            while !current_node.is_null() {
                unsafe {
                    new_map.insert((*current_node).key.clone(), (*current_node).value.clone());
                    current_node = (*current_node).next;
                }
            }
        }
        new_map
    }
}
impl<
    K: core::cmp::Eq + core::hash::Hash + core::clone::Clone,
    V: core::clone::Clone,
    S: core::hash::BuildHasher,
> Drop for HashMap<K, V, S>
{
    fn drop(&mut self) {
        self.clear()
    }
}
#[derive(Clone, Copy, Debug)]
pub struct FnvHasher(u64);

impl Hasher for FnvHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= byte as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3); // FNV prime
        }
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0 // Returns the computed hash value
    }
}

// 2. The BuildHasher (S) is the state factory that creates a clean Hasher for each operation
#[derive(Clone, Copy, Debug, Default)]
pub struct FnvBuildHasher;

impl BuildHasher for FnvBuildHasher {
    type Hasher = FnvHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        FnvHasher(0xcbf29ce484222325) // FNV offset basis
    }
}

struct ValuesIter<'a, K: Hash + Eq + Clone, V: Clone, S: BuildHasher> {
    map: &'a HashMap<K, V, S>,
    bucket: usize,
    current: *mut HashNode<K, V>,
}

impl<'a, K, V, S> Iterator for ValuesIter<'a, K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: BuildHasher,
{
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current.is_null() {
            if self.bucket >= BUCKET_COUNT {
                return None;
            }

            self.current = self.map.buckets[self.bucket];
            self.bucket += 1;
        }

        unsafe {
            let node = &*self.current;
            self.current = node.next;
            Some(&node.value)
        }
    }
}

impl<K, V, S> HashMap<K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: BuildHasher,
{
    pub fn values(&self) -> impl Iterator<Item = &V> + '_ {
        ValuesIter {
            map: self,
            bucket: 0,
            current: null_mut(),
        }
    }
}
