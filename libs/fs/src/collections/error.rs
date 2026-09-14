use core::{error::Error, fmt::Display};

use alloc::string::{String, ToString};
use block::error::BlockError::{self};

#[derive(Clone, Debug)]
pub enum FSError {
    /// File not found, `String` is path
    FileNotFound(String),
    Read(&'static str),
    Type(String),
    /// Low level block driver error
    IOError(&'static str),
    VolumeFull(&'static str),
    Resolve(String),
    OutOfBounds(String),
    Corrupted(&'static str),
    PermissionDenied {
        permission: &'static str,
        needs: &'static str,
    },
    Other,
}

impl FSError {
    pub fn file_not_found(path: String) -> Self {
        Self::FileNotFound(path)
    }
}
impl From<BlockError> for FSError {
    fn from(value: BlockError) -> Self {
        match value {
            BlockError::DeviceError(err) => Self::IOError(err),
            BlockError::AlignError(msg)
            | BlockError::BufferMismatch(msg)
            | BlockError::OutOfBounds(msg) => Self::Type(msg.to_string()), // TODO: Make this comprehensive
            _ => Self::Other,
        }
    }
}

impl Display for FSError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FileNotFound(path) => f.write_fmt(format_args!("File not found: {}", path)),
            Self::Read(msg) => f.write_fmt(format_args!("Read error: {}", msg)),
            _ => f.write_str("Type error"),
        }
    }
}
