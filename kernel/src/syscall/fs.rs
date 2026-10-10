use core::str::{self, FromStr};

use alloc::{borrow::ToOwned, string::String};
use fs::{collections::error::FSError, core::path::FSPath};
use log::{error, warn};

use crate::syscall::{
    SyscallArgs,
    context::KernelContext,
    error::SyscallError,
    op::{SysOp, exit, str_from_args},
};

/// File system operations
pub enum FsOp {
    Open,
    Read,
    Write,
    Close,
    Seek,
}

impl SysOp for FsOp {
    fn call(&self, ktx: &mut KernelContext, args: SyscallArgs) -> Result<(), SyscallError> {
        match self {
            Self::Open => unsafe {
                let (path_ptr, path_len, ..) = args;
                let path = str_from_args(path_ptr, path_len)?;
                let fspath = FSPath::new(&path);
                open(ktx, file_path) {
                    Ok(_) => exit(ktx, 0),
                    Err(msg) => {
                        error!("{msg}");
                        exit(ktx, -1);
                    }
                }
            },
            Self::Close => {
                let path = Self::get_path(args.0, args.1);
            }
            _ => (),
        }
    }
}
impl TryFrom<usize> for FsOp {
    type Error = SyscallError;
    fn try_from(value: usize) -> Result<Self, super::error::SyscallError> {
        Ok(match value {
            0 => Self::Open,
            1 => Self::Read,
            2 => Self::Write,
            3 => Self::Close,
            4 => Self::Seek,
            _ => return Err(SyscallError::InvalidOperation("FsOp", value)),
        })
    }
}
