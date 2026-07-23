use core::str;

use crate::syscall::SyscallArgs;

/// File system operations
pub enum FsOp {
    Open,
    Read,
    Write,
    Close,
    Seek,
}

impl FsOp {
    fn call(args: SyscallArgs) {
        match Self {
            Self::Open => {
                let path = Self::get_path(args.0, args.1);
                open(path);
            }
            Self::Close => {
                let path = Self::get_path(addr, len)
            }
            _ => (),
        }
    }
    fn get_path(addr: usize, len: usize) -> &str {
        unsafe { str::from_raw_parts(args.0 as *const u8, args.1) }
    }
}

impl From<usize> for FsOp {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Open,
            1 => Self::Read,
            2 => Self::Write,
            3 => Self::Close,
            4 => Self::Seek,
        }
    }
}

/// Opens a file for reading or writing (text, audio, etc.).
fn open(file_path: &str) {}
// Reads data from an opened file.
fn read(file_path: &str) {}
// Writes or saves data to a file.

// Closes an opened file.
// Moves the file pointer to a specific position in a file (e.g., jump to line 47 to read from there).
