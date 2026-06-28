use core::{error, fmt};
/// reading fdt tables
#[derive(Clone, Debug)]
pub enum SdtError {
    ParseError(&'static str),
    ValidationError(&'static str),
}

impl fmt::Display for SdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SdtError::ParseError(msg) => write!(f, "Failed to parse FDT: {}", msg),
            SdtError::ValidationError(msg) => write!(f, "Failed to validate FDT: {}", msg),
        }
    }
}

impl error::Error for SdtError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}
