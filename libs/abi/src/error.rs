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
