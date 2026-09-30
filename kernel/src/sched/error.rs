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
