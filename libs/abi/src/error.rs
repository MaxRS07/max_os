use core::fmt::Display;

use alloc::string::String;

#[repr(isize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Errno {
    InvalidCall = -1,
    InvalidOp = -2,
    InvalidArg = -3,
    Busy = -4,
    Failed = -5,
}

impl Errno {
    pub fn from_raw(value: isize) -> Self {
        match value {
            -1 => Self::InvalidCall,
            -2 => Self::InvalidOp,
            -3 => Self::InvalidArg,
            -4 => Self::Busy,
            -5 | _ => Self::Failed, // todo: phase out this error
        }
    }
}
