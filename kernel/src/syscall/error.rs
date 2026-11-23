use core::fmt::Display;

use abi::error::Errno;
use alloc::string::String;

use crate::syscall::Syscall;

pub enum SyscallError {
    InvalidCall(usize),
    InvalidOperation(&'static str, usize),
    Busy(String),
    InvalidArg(String),
    Failed(String),
}

impl SyscallError {
    pub fn errno(&self) -> Errno {
        match self {
            Self::InvalidCall(_) => Errno::InvalidCall,
            Self::InvalidOperation(..) => Errno::InvalidOp,
            Self::Busy(_) => Errno::Busy,
            Self::InvalidArg(_) => Errno::InvalidArg,
            Self::Failed(_) => Errno::Failed,
        }
    }
}

impl Display for SyscallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidCall(call) => f.write_fmt(format_args!("Invalid Call: call type {call}")),
            Self::InvalidOperation(name, num) => {
                f.write_fmt(format_args!("Invalid Operation: operation {num} on {name}"))
            }
            Self::Busy(msg) => f.write_str(msg),
            Self::InvalidArg(msg) => f.write_str(msg),
            Self::Failed(msg) => f.write_str(msg),
        }
    }
}

impl From<Errno> for SyscallError {
    fn from(value: Errno) -> Self {
        match value {
            Errno::Busy => Self::Busy(),
        }
    }
}
