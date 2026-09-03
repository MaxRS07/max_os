use core::{clone::Clone, cmp::Eq, hash::Hash};

use alloc::collections::LinkedList;

use crate::hashmap::HashMap;

pub struct LRUCache<K: Hash + Eq + Clone, V: Clone> {
    map: HashMap<K, V>,
    queue: LinkedList<K>,
    len: usize,
    capacity: usize,
}

impl<K: Hash + Eq + Clone, V: Clone> LRUCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            queue: LinkedList::new(),
            len: 0,
            capacity,
        }
    }
    /// Removes a node from the lru cache, returning Some(value) if found, None otherwise
    pub fn remove_node(&mut self, node: &K) -> Option<V>
    where
        K: Eq + Clone,
    {
        if self.queue.extract_if(|item| item == node).next().is_some() {
            let val = self.map.remove(node);
            self.len -= 1;
            return val;
        }
        // not present
        None
    }
    fn move_to_head(&mut self, node: K) -> Option<V> {
        let val = self.remove_node(&node)?;
        self.queue.push_front(node);
        Some(val)
    }
    pub fn get(&mut self, key: K) -> Option<V> {
        if let Some(value) = self.map.get(&key) {
            let _ = self.move_to_head(key);
            return Some(value);
        }
        None
    }
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        if self.len == self.capacity {
            self.remove_last()?;
        }
        self.map.insert(key.clone(), value.clone()).map(|value| {
            self.queue.push_front(key);
            self.len += 1;
            Some(value)
        })?
    }
    fn remove_last(&mut self) -> Option<K> {
        if let Some(last) = self.queue.pop_back() {
            self.map.remove(&last);
            return Some(last);
        }
        None
    }
}
