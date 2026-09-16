use core::str::{self, FromStr};

use alloc::{borrow::ToOwned, string::String};
use log::warn;

use crate::syscall::{SyscallArgs, context::KernelContext, op::SysOp};

/// File system operations
pub enum FsOp {
    Open,
    Read,
    Write,
    Close,
    Seek,
}

impl SysOp for FsOp {
    fn call(&self, ktx: KernelContext, args: SyscallArgs) {
        match self {
            Self::Open => {
                let path = Self::get_path(args.0, args.1);
            }
            Self::Close => {
                let path = Self::get_path(args.0, args.1);
            }
            _ => (),
        }
    }
}
impl FsOp {
    fn get_path(addr: usize, len: usize) -> String {
        unsafe {
            let str_ptr = addr as *const u8;
            let bytes = core::slice::from_raw_parts(str_ptr, len);
            let str = str::from_utf8(bytes).unwrap(); // TODO: Actually handle this 
            String::from_str(str).unwrap()
        }
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
            _ => panic!("Unsupported file system operation"),
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
