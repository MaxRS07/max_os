use core::{
    error::Error,
    fmt::{Display, write},
    ops::MulAssign,
};

#[derive(Clone, Copy, Debug)]
pub enum MemoryError {
    OutOfMemory(&'static str),
    InvalidAddress(&'static str),
    AccessViolation(&'static str),
    PageFault(&'static str),
    StackOverflow(&'static str),
    HeapCorruption(&'static str),
    MemoryLeak(&'static str),
    AlignmentError(&'static str),
    ProtectionViolation(&'static str),
}

impl Display for MemoryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutOfMemory(msg) => write!(f, "Out of memory error: {}", msg),
            Self::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            Self::AccessViolation(msg) => {
                write!(f, "Insufficient permissions to access memory: {}", msg)
            }
            Self::PageFault(msg) => write!(f, "Page fault exception: {}", msg),
            Self::StackOverflow(msg) => write!(f, "Stack overflow: {}", msg),
            Self::HeapCorruption(msg) => write!(f, "Heap corrupted: {}", msg),
            Self::MemoryLeak(msg) => write!(f, "Memory leak: {}", msg),
            Self::AlignmentError(msg) => write!(f, "Memory misaligned to type: {}", msg),
            Self::ProtectionViolation(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error for MemoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
