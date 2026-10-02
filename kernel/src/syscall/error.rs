use core::fmt::Display;

use alloc::string::String;

pub enum SyscallError {
    InvalidCall,
    InvalidOperation,
    Busy(String),
}

impl Display for SyscallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidCall => f.write_fmt(format_args!("Invalid Call")),
            Self::InvalidOperation => f.write_fmt(format_args!("Invalid Operation")),
            Self::Busy(msg) => f.write_str(msg),
        }
    }
}
