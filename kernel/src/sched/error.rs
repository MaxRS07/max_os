use core::fmt::Display;

use alloc::{
    format,
    string::{String, ToString},
};

use crate::sched::thread::thread::ThreadId;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum SchedError {
    BadPid(u32),
    BadTid(ThreadId),
    Other(String),
    InvalidFileDescriptor,
    FileDescriptorsFull(String),
    BadFileDescriptor(u32),
    OutOfResources(String),
}

impl Display for SchedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Other(msg) => f.write_str(msg),
            Self::BadPid(pid) => f.write_fmt(format_args!("Invalid PID: {pid}, process not found")),
            Self::BadTid(thread_id) => f.write_fmt(format_args!(
                "Invalid thread id: {thread_id:?}, thread not found"
            )),
            Self::FileDescriptorsFull(msg) => f.write_str(msg),
            Self::InvalidFileDescriptor => f.write_str("Invalid file descriptor"),
            Self::OutOfResources(msg) => f.write_str(msg),
            Self::BadFileDescriptor(fd) => {
                f.write_str(&format!("File descriptor {fd} not open for process"))
            }
        }
    }
}
