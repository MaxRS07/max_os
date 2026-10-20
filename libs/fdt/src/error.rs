use core::{error, fmt};
/// reading fdt tables
#[derive(Clone, Debug)]
pub enum FdtError {
    ParseError(&'static str),
    ValidationError(&'static str),
}

impl fmt::Display for FdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FdtError::ParseError(msg) => write!(f, "Failed to parse FDT: {}", msg),
            FdtError::ValidationError(msg) => write!(f, "Failed to validate FDT: {}", msg),
        }
    }
}

impl error::Error for FdtError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}
