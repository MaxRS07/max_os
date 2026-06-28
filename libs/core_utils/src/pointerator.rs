/// Iterates over a chunk of memory. This seems like a bad idea but its my OS
/// TODO: Add support for taking byte array and endian-specific support
pub struct Pointerator {
    start_addr: *const u8,
    end_addr: *const u8,
}

impl Pointerator {
    /// Creates a Pointerator!
    /// ## Parameters
    /// `start_addr`: address to start of iterator chunk\
    /// `length`: size of chunk in bytes
    /// ## Safety
    /// Please dont let size exceed program or kernel will crash
    pub unsafe fn new(start_addr: *const u8, length: usize) -> Self {
        let end_addr = unsafe { start_addr.add(length) };
        Self {
            start_addr,
            end_addr,
        }
    }
    /// Reads the current pointer value as `T` in unaligned native-endian format, advances pointer by size of `T`
    /// ## Returns
    /// `Some(T)` *if you're lucky*\
    /// `None` if next address exceeds the available memory
    /// ## Safety
    /// This is safeish
    pub unsafe fn next<T>(&mut self) -> Option<T> {
        let size = core::mem::size_of::<T>();
        unsafe {
            if (self.start_addr.add(size)) > self.end_addr {
                return None;
            }
            let t_addr = self.start_addr as *const T;

            let value = t_addr.read_unaligned();
            self.start_addr = self.start_addr.add(size);
            Some(value)
        }
    }
}
