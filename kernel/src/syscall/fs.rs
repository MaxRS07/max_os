use core::str::{self, FromStr};

use alloc::{
    borrow::ToOwned,
    fmt::format,
    format,
    string::{String, ToString},
};
use fs::{
    collections::error::FSError,
    core::path::FSPath,
    storage::fileobject::{FileObject, OpenMode, SeekMode},
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
                let (fd, buf_addr, buf_len, ..) = args;
                let process = ktx.process()?;
                let fo = process
                    .file_mut(fd as u32)
                    .map_err(|err| SyscallError::Failed(err.to_string()))?;
                let mut vfs = ktx.vfs()?;
                let volume = vfs
                    .get_volume(fo.volume_id())
                    .ok_or(SyscallError::Failed(format!("Invalid volume id")))?;
                let buf_ptr = buf_addr as *mut u8;
                let buf = core::slice::from_raw_parts_mut(buf_ptr, buf_len);
                let rlen = volume
                    .read(fo, buf)
                    .map_err(|err| SyscallError::Failed(err.to_string()))?;
                write_return!(ktx, rlen);
                return Ok(());
            },
            Self::Write => unsafe {
                let (fd, buf_addr, buf_len, ..) = args;
                let process = ktx.process()?;
                let fo = process
                    .file_mut(fd as u32)
                    .map_err(|err| SyscallError::Failed(err.to_string()))?;
                let mut vfs = ktx.vfs()?;
                let volume = vfs
                    .get_volume(fo.volume_id())
                    .ok_or(SyscallError::Failed(format!("Invalid volume id")))?;
                let buf_ptr = buf_addr as *const u8;
                let buf = core::slice::from_raw_parts(buf_ptr, buf_len);
                volume
                    .write(fo, buf)
                    .map_err(|err| SyscallError::Failed(format!("{err}")))?;
                return Ok(());
            },
            // close the file descriptor for this process
            Self::Close => {
                let (fd, ..) = args;
                let process = ktx.process()?;
                process
                    .close_fd(fd as u32)
                    .map_err(|err| SyscallError::Failed(format!("{err}")))?;
                return Ok(());
            }
            Self::Seek => {
                let (fd, offset, mode, ..) = args;
                let process = ktx.process()?;
                let fo = process
                    .file_mut(fd as u32)
                    .map_err(|err| SyscallError::Failed(err.to_string()))?;
                let seek_mode = match mode {
                    0 => SeekMode::Set,
                    1 => SeekMode::Current,
                    2 => SeekMode::End,
                    _ => {
                        return Err(SyscallError::InvalidArg(format!(
                            "Invalid seek mode ({mode}). Mode must be less than 3"
                        )));
                    }
                };
                match fo.seek(offset, seek_mode) {
                    Ok(cursor) => {
                        write_return!(ktx, cursor);
                        Ok(())
                    }
                    Err(err) => Err(SyscallError::Failed(format!("{err}"))),
                }
            }
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

fn open(ktx: &mut KernelContext, path: &str, mode: u8) -> Result<FileObject, SyscallError> {
    let mut vfs = ktx.vfs()?;
    let mode = OpenMode::from(mode);
    let fspath = FSPath::new(path);
    vfs.open(fspath, mode)
        .map_err(|msg| SyscallError::InvalidArg(format!("Failed to open file: {msg}")))
}
