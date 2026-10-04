use core::fmt::Display;

use alloc::string::String;

pub enum SyscallError {
    InvalidCall(usize),
    InvalidOperation(&'static str, usize),
    Busy(String),
}

impl Display for SyscallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidCall(call) => f.write_fmt(format_args!("Invalid Call: call type {call}")),
            Self::InvalidOperation(name, num) => {
                f.write_fmt(format_args!("Invalid Operation: operation {num} on {name}"))
            }
            Self::Busy(msg) => f.write_str(msg),
        }
    }
}
