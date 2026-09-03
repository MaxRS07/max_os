use core::fmt::Display;

pub enum BlockError {
    DeviceError(&'static str),
    BufferMismatch(&'static str),
    OutOfBounds(&'static str),
    AlignError(&'static str),
    Readonly,
    Other,
}

impl Display for BlockError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DeviceError(msg) => f.write_fmt(format_args!("Device Error: {}", msg)),
            Self::BufferMismatch(msg) => f.write_fmt(format_args!("Device Error: {}", msg)),
            Self::OutOfBounds(msg) => f.write_fmt(format_args!("Device Error: {}", msg)),
            Self::AlignError(msg) => f.write_fmt(format_args!("{}", msg)),
            Self::Readonly => f.write_str("Attempted to mutate a readonly sector"),
            Self::Other => f.write_str("Other"),
        }
    }
}
