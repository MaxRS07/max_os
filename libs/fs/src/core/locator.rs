use crate::core::path::FSPath;

/// Common trait for types that can be used as file system locators
pub trait FSLocator {
    /// Returns each component in the path. Absolute paths begin with `"/"`.
    fn components(&self) -> impl Iterator<Item = &str>;
    /// `true` if this path begins at the root folder
    fn is_absolute(&self) -> bool;
    /// returns the file extension of file at the path, or an empty str `""` if the path is a directory or no extension is found
    fn extension(&self) -> &str;
    /// Returns the path to the parent directory of this as a new `Self`
    fn parent(&self) -> &Self;
    /// Checks if a str is in valid path format
    fn is_valid(&self) -> bool;
    /// Returns a path of the first `depth` components of self.
    /// When `depth` is `0` returns `"/"`
    ///
    /// # Examples
    /// ```
    /// let path = FSPath("a/b/c/d");
    /// assert_eq!(path.root(3), "a/b/c")
    /// let path = FSPath("/a/b/c/d");
    /// assert_eq!(path.root(2), "/a/b")
    /// ```
    fn prefix(&self, depth: usize) -> &Self;
    fn matches_prefix(&self, other: &Self) -> bool;
    /// Gets the last component of the path
    ///
    /// # Examples
    /// ```
    /// let path = FSPath("user/desktop");
    /// assert_eq!(path.name(), "desktop")
    /// let path = FSPath("user/desktop/file.txt");
    /// assert_eq!(path.name(), "file.text")
    /// ```
    fn name(&self) -> &str;
}
