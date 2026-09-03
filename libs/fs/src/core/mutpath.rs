use alloc::{borrow::ToOwned, boxed::Box, fmt::format, string::String, vec::Vec};

use crate::core::{locator::FSLocator, path::FSPath};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FSMutPath {
    path: String,
}
impl FSMutPath {
    pub fn from_slice(path: &str) -> Self {
        Self {
            path: path.to_owned(),
        }
    }
    pub fn from_string(path: String) -> Self {
        Self { path }
    }
    pub fn empty() -> Self {
        Self {
            path: String::new(),
        }
    }
    pub fn as_path(&self) -> &FSPath {
        &FSPath::new(&self.path)
    }
    // /// Joins this directory path with another `MutPath`, returning the new `MutPath` object. Returns `None` if this path is to a file, in which case it cannot be joined.
    // pub fn join(&self, other: Self) -> Option<Self> {
    //     let mut joined = String::new();
    //     if self.is_absolute() {
    //         joined.push('/');
    //     }
    //     for c in self.components() {
    //         joined.push_str(c);
    //         joined.push('/');
    //     }
    //     for c in other.components() {
    //         joined.push_str(c);
    //         joined.push('/');
    //     }
    //     Some(Self::from_string(joined))
    // }
    // pub fn join_string(&self, other: String) -> Option<Self> {
    //     let other = Self::from_string(other);
    //     self.join(other)
    // }
    // pub fn join_str(&self, other: &str) -> Option<Self> {
    //     let other = Self::from_slice(other);
    //     self.join(other)
    // }
}
