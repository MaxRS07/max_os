use core::fmt::Display;

pub enum SyscallError {
    InvalidCall,
    InvalidOperation,
}

impl Display for SyscallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidCall => f.write_fmt(format_args!("Invalid Call")),
            Self::InvalidOperation => f.write_fmt(format_args!("Invalid Operation")),
        }
    }
}
