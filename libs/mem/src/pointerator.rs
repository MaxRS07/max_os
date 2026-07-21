/// Iterates over a chunk of memory. This seems like a bad idea but its my OS
/// TODO: Add support for taking byte array and endian-specific support
pub struct Pointerator {
    cur_ptr: *const u8,
    end_ptr: *const u8,
}

impl Pointerator {
    /// Creates a Pointerator
    /// ## Parameters
    /// `start_addr`: pointer to start of iterator chunk\
    /// `length`: size of chunk in bytes
    /// ## Safety
    /// Please dont let size exceed program or kernel will crash
    pub unsafe fn from_ptr(cur_ptr: *const u8, length: usize) -> Self {
        let end_ptr = unsafe { cur_ptr.add(length) };
        Self { cur_ptr, end_ptr }
    }
    /// Creates a Pointerator
    /// ## Parameters
    /// `start_addr`: address of start of iterator chunk\
    /// `length`: size of chunk in bytes
    /// ## Safety
    /// Please dont let size exceed program or kernel will crash
    pub fn from_addr(start_addr: usize, length: usize) -> Self {
        let cur_ptr = start_addr as *const u8;
        let end_ptr = unsafe { cur_ptr.add(length) };
        Self { cur_ptr, end_ptr }
    }
    /// Reads the current pointer value as `T` in unaligned native-endian format, advances pointer by size of `T`
    /// ## Returns
    /// `Some(T)` or\
    /// `None` if next address exceeds the available memory
    /// ## Safety
    pub unsafe fn next<T>(&mut self) -> Option<T> {
        let size = core::mem::size_of::<T>();
        unsafe {
            if (self.cur_ptr.add(size)) > self.end_ptr {
                return None;
            }
            let t_addr = self.cur_ptr as *const T;

            let value = t_addr.read_unaligned();
            self.cur_ptr = self.cur_ptr.add(size);
            Some(value)
        }
    }
    /// Reads the current pointer value as `T` in stack format, decrements pointer by size of `T`
    /// ## Returns
    /// `Some(T)` or\
    /// `None` if next address exceeds the available memory
    /// ## Safety
    pub unsafe fn stack_next<T>(&mut self) -> Option<T> {
        let size = core::mem::size_of::<T>();
        unsafe {
            if (self.cur_ptr.add(size)) > self.end_ptr {
                return None;
            }
            let t_addr = self.cur_ptr as *const T;

            let value = t_addr.read_unaligned();
            self.cur_ptr = self.cur_ptr.add(size);
            Some(value)
        }
    }

    /// Reads the current pointer value as `T` in unaligned native-endian format, advances pointer by size of `T`
    /// ## Returns
    /// `Some(T)` or\
    /// `None` if next address exceeds the available memory
    /// ## Safety
    pub unsafe fn next_mut<T>(&mut self) -> Option<&mut T> {
        let size = core::mem::size_of::<T>();
        unsafe {
            if (self.cur_ptr.add(size)) > self.end_ptr {
                return None;
            }
            let t_addr = self.cur_ptr as *mut T;
            let value = &mut *t_addr;
            self.cur_ptr = self.cur_ptr.add(size);
            Some(value)
        }
    }
    /// Reads the current pointer value as `T` in unaligned native-endian format, advances pointer by size of `T`
    /// ## Returns
    /// `Some(T)` or\
    /// `None` if next address exceeds the available memory
    /// ## Safety
    pub unsafe fn stack_next_mut<T>(&mut self) -> Option<&mut T> {
        let size = core::mem::size_of::<T>();
        unsafe {
            if (self.cur_ptr.sub(size)) > self.end_ptr {
                return None;
            }
            let t_addr = self.cur_ptr as *mut T;
            let value = &mut *t_addr;
            self.cur_ptr = self.cur_ptr.add(size);
            Some(value)
        }
    }
}
