use core::iter::Peekable;
use core::panic;

use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};
use alloc::vec::{self, Vec};
use collections::hashmap::HashMap;

use crate::core::locator::FSLocator;
use crate::core::mutpath::FSMutPath;
use crate::core::path::FSPath;

pub type NodeId = usize;

#[derive(Debug, Clone)]
pub struct FSNode {
    name: String,
    parent: Option<NodeId>,
    // Mapping from "child name" to its index in the arena
    children: HashMap<String, NodeId>,
    size: usize,
}

impl FSNode {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }
    pub fn set_parent(&mut self, value: usize) {
        self.parent = Some(value)
    }
}

#[derive(Debug)]
pub struct FSRouteTrie {
    /// all nodes in sequential memory.
    nodes: Vec<FSNode>,
    /// indices of `FSNode::None`
    free: Vec<NodeId>,
    root: NodeId,
}

impl FSRouteTrie {
    pub fn new() -> Self {
        let root_node = FSNode {
            name: "/".to_string(),
            parent: None,
            children: HashMap::new(),
            size: 0,
        };

        Self {
            nodes: alloc::vec![root_node],
            free: Vec::new(),
            root: 0,
        }
    }

    fn path_components(&self, path: &FSPath) -> Option<Vec<String>> {
        if !path.is_absolute() {
            return None;
        }

        Some(
            path.components()
                .skip(path.is_absolute() as usize)
                .map(|cmp| cmp.to_owned())
                .collect(),
        )
    }

    fn child_idx(&self, parent_idx: NodeId, name: &str) -> Option<NodeId> {
        self.nodes.get(parent_idx)?.children.get(&name.to_owned())
    }
    /// Inserts a path and returns the NodeId of the final inserted component.
    pub fn insert(&mut self, path: &FSPath) -> Option<NodeId> {
        let components = self.path_components(path)?;
        if components.is_empty() {
            return Some(self.root);
        }

        let mut current_idx = self.root;
        for cmp in components.iter() {
            let name_str = cmp.to_owned();

            if let Some(child_idx) = self.child_idx(current_idx, &name_str) {
                current_idx = child_idx;
                continue;
            }

            let next_idx = self.nodes.len();

            let mut new = FSNode {
                name: name_str.clone(),
                parent: None,
                children: HashMap::empty(),
                size: 0,
            };
            new.set_parent(current_idx);
            self.add_node(new);

            self.nodes
                .get_mut(current_idx)?
                .children
                .insert(name_str, next_idx);

            current_idx = next_idx;
        }

        Some(current_idx)
    }

    /// Adds a node to the list. Will try to reuse free node indices first.
    fn add_node(&mut self, node: FSNode) {
        if let Some(idx) = self.free.pop() {
            self.nodes[idx] = node;
            return;
        }
        self.nodes.push(node);
    }

    /// Safely retrieve a node reference via its ID
    pub fn get(&self, id: NodeId) -> Option<&FSNode> {
        self.nodes.get(id)
    }

    pub fn find(&self, path: &FSPath) -> Option<NodeId> {
        let components = self.path_components(path)?;
        if components.is_empty() {
            return Some(self.root);
        }

        let mut current_idx = self.root;

        for cmp in components {
            let child_idx = self.child_idx(current_idx, &cmp)?;
            current_idx = child_idx;
        }

        Some(current_idx)
    }

    pub fn path(&self, id: NodeId) -> Option<FSMutPath> {
        let mut cur_id = Some(id);
        let mut path_cmps: Vec<&str> = alloc::vec![];
        while let Some(_id) = cur_id {
            let cur = self.get(_id)?;
            // ok to unwrap, cur is not none
            path_cmps.push(cur.name());
            cur_id = cur.parent();
        }
        path_cmps.reverse();
        let path_str = path_cmps.join("/");
        let path = FSMutPath::from_string(path_str);
        Some(path)
    }

    pub fn remove(&mut self, id: NodeId) {
        let Some(node) = self.nodes.get(id) else {
            return;
        };
        let child_ids: Vec<_> = node.children.values().copied().collect();

        for child_id in child_ids {
            self.remove(child_id);
        }
        self.free.push(id);
    }
}

impl Default for FSRouteTrie {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for FSRouteTrie {}
unsafe impl Sync for FSRouteTrie {}
