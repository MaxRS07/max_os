use core::{error::Error, fmt::Display};

use alloc::string::String;
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
    OutOfBounds(&'static str),
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
            _ => Self::Other, // TODO: Make this comprehensive
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
