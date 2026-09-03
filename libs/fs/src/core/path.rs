use core::{
    cmp::min,
    ops::{Index, Range, RangeTo},
    panic,
};

use alloc::{borrow::ToOwned, string::String};

use crate::core::{locator::FSLocator, mutpath::FSMutPath};

pub const MAX_PATH: usize = 0xFF; // 255 chars for max path length

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FSPath {
    path: str,
}

impl FSPath {
    pub fn new(path: &str) -> &Self {
        let new = unsafe { &*(path as *const str as *const FSPath) };
        if !new.is_valid() {
            return Self::empty();
        }
        new
    }
    /// Returns a static reference to an empty `FSPath`
    pub fn empty() -> &'static Self {
        unsafe { &*("" as *const str as *const FSPath) }
    }

    pub fn to_owned(&self) -> FSMutPath {
        FSMutPath::from_slice(&self.path)
    }
    pub fn is_root(&self) -> bool {
        self.path.eq("/")
    }
    pub fn len(&self) -> usize {
        self.path.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Index<Range<usize>> for FSPath {
    type Output = str;

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.path[index.start..index.end]
    }
}
impl Index<RangeTo<usize>> for FSPath {
    type Output = str;

    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self.path[..index.end]
    }
}
impl Index<usize> for FSPath {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        &self.path[index..=index]
    }
}

impl PartialEq<str> for FSPath {
    fn eq(&self, other: &str) -> bool {
        self.path.eq(other)
    }
}

impl FSLocator for FSPath {
    fn components(&self) -> core::str::Split<'_, &str> {
        self.path.split("/")
    }

    fn is_absolute(&self) -> bool {
        self.path.starts_with('/')
    }

    fn extension(&self) -> &str {
        if let Some(tail) = self.components().last() {
            if !tail.contains('.') {
                return "";
            } else {
                return tail.split('.').next_back().unwrap_or("");
            }
        }
        ""
    }

    fn parent(&self) -> &Self {
        let parent_path = self.path.rsplit_once('/').unwrap_or(("", "")).0;
        Self::new(parent_path)
    }

    fn is_valid(&self) -> bool {
        let len = self.components().count();
        for (i, c) in self.components().enumerate() {
            let first_or_last = i == 0 || i == len - 1;
            if (!first_or_last && c.is_empty()) || !c.is_ascii() || c.trim().len() != c.len() {
                return false;
            }
        }
        true
    }
    fn name(&self) -> &str {
        self.components().last().unwrap_or("")
    }

    fn prefix(&self, depth: usize) -> &Self {
        if depth > self.components().count() {
            panic!("Maximum depth exceeded")
        }
        if depth == 0 {
            return FSPath::new("/");
        }
        let mut nth = 0;
        let mut idx = 0;
        for (i, c) in self.path.chars().enumerate() {
            // exclude leading /
            if c == '/' && i != 0 {
                nth += 1;
                if nth == depth {
                    idx = i;
                    break;
                }
            }
        }
        let path = &self.path[..idx];
        Self::new(path)
    }
    /// checks if paths have the same starting components up to the depth of the shorter path. Paths will only match if they have the same absoluteness.
    /// # Example
    /// ```
    /// let path_1 = FSPath::new("/foo/bar/");
    /// let path_2 = FSPath::new("/foo/");
    /// assert!(path_1.matches_prefix(path_2))
    ///
    /// let path_1 = FSPath::new("/foo/bar/");
    /// let path_2 = FSPath::new("foo/");
    /// assert!(!path_1.matches_prefix(path_2))
    ///
    /// let path_1 = FSPath::new("/foo/bar/");
    /// let path_2 = FSPath::new("/");
    /// assert!(path_1.matches_prefix(path_2))
    /// ```
    fn matches_prefix(&self, other: &Self) -> bool {
        if other.is_absolute() != self.is_absolute() {
            return false;
        }
        let mut self_cmp = self.components();
        let mut other_cmp = other.components();

        loop {
            match (self_cmp.next(), other_cmp.next()) {
                (Some(s), Some(o)) if s == o => continue,
                (_, None) | (None, _) => return true,
                _ => return false,
            }
        }
    }
}

impl AsRef<str> for FSPath {
    fn as_ref(&self) -> &str {
        &self.path
    }
}

impl From<&FSPath> for String {
    fn from(val: &FSPath) -> Self {
        val.path.to_owned()
    }
}
