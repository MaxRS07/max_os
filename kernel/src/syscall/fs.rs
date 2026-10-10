use core::str::{self, FromStr};

use alloc::{borrow::ToOwned, fmt::format, format, string::String};
use fs::{
    collections::error::FSError,
    core::path::FSPath,
    storage::fileobject::{FileObject, OpenMode},
};
use log::{error, warn};

use crate::{
    arch::riscv::mode,
    syscall::{
        SyscallArgs,
        context::KernelContext,
        error::SyscallError,
        op::{SysOp, exit, str_from_args},
    },
    write_return,
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
            // open a file descriptor on the current process
            Self::Open => unsafe {
                let (path_ptr, path_len, mode, ..) = args;
                let path = str_from_args(path_ptr, path_len)?;
                let fo = open(ktx, &path, mode as u8)?;
                let process = ktx.process()?;
                let fd = process
                    .alloc_fd(fo)
                    .map_err(|err| SyscallError::Failed(format!("{err}")))?;
                // write the file desc to a1
                write_return!(ktx, fd as isize);
                return Ok(());
            },
            Self::Read => unsafe {
                let (fd, buf, buf_len, ..) = args;
                let process = ktx.process()?;
                let fo = process
                    .file_mut(fd as u32)
                    .ok_or(SyscallError::InvalidArg(format!(
                        "Failed to read file, invalid file descriptor"
                    )))?;
                let volume = ktx
                    .vfs()?
                    .get_volume(fo.volume_id())
                    .ok_or(SyscallError::Failed(format!("Invalid volume id")))?;
                let buf = bud as *mut [u8; buf_size];
                let rlen = volume.read(f, &mut *buf)?;
                write_return!(ktx, rlen);
                return Ok(());
            },
            Self::Write => unsafe {
                let (fd, buf, buf_len, ..) = args;
                let process = ktx.process()?;
                let fo = process
                    .file_mut(fd as u32)
                    .ok_or(SyscallError::InvalidArg(format!(
                        "Failed to read file, invalid file descriptor"
                    )))?;
                let volume = ktx
                    .vfs()?
                    .get_volume(fo.volume_id())
                    .ok_or(SyscallError::Failed(format!("Invalid volume id")))?;
                let buf = bud as *mut [u8; buf_size];
                volume.write(f, &*buf)?;
                return Ok(());
            },
            // close the file descriptor for this process
            Self::Close => {
                let (fd, ..) = args;
                let process = ktx.process()?;
                process.close_fd(fd as u32);
                return Ok(());
            }
            _ => Err(SyscallError::InvalidCall(0)),
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

fn open(
    ktx: &mut KernelContext,
    path: &str,
    mode: u8,
) -> Result<FileObject<'static, 'static>, SyscallError> {
    let vfs = ktx.vfs()?;
    let mode = OpenMode::from(mode);
    let fspath = FSPath::new(path);
    vfs.open(fspath, mode)
        .map_err(|msg| SyscallError::InvalidArg(format!("Failed to open file: {msg}")))
}
