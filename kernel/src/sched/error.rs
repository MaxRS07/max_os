use core::fmt::Display;

use alloc::string::String;

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
    NullFileDescriptor(String),
    FileDescriptorsFull(String),
}

impl Display for ProcessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FileDescriptorsFull(msg) => f.write_str(msg),
            Self::NullFileDescriptor(msg) => f.write_str(msg),
        }
    }
}
