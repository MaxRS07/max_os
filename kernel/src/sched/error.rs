use core::fmt::Display;

use alloc::{format, string::String};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ThreadError {
    Other(String),
}

impl Display for ThreadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Other(msg) => f.write_str(msg),
        }
    }
}

pub enum ProcessError {
    InvalidFileDescriptor,
    FileDescriptorsFull(String),
    BadFileDescriptor(u32),
    OutOfResources(String),
}

impl Display for ProcessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FileDescriptorsFull(msg) => f.write_str(msg),
            Self::InvalidFileDescriptor => f.write_str("Invalid file descriptor"),
            Self::OutOfResources(msg) => f.write_str(msg),
            Self::BadFileDescriptor(fd) => {
                f.write_str(&format!("File descriptor {fd} not open for process"))
            }
        }
    }
}
