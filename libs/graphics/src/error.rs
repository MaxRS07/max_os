use core::{
    fmt::{Display, Write},
    todo,
};

#[derive(Debug)]
pub enum DisplayError {
    // surface errors
    InvalidSurface,
    BadOwner(u32, u32),
    /// actual size, min size
    InsufficientBacking(usize, usize),
    Unsupported,
    AllocationFailed,
    SurfacesExhausted,

    AttachBackingFailed,
    ScanoutFailed,

    // per-frame errors
    TransferFailed,
    FlushFailed,
    /// catch all device errors here
    DeviceError(&'static str),
}

impl core::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSurface => f.write_str("Invalid surface for "),
            Self::BadOwner(pid, owner) => f.write_fmt(format_args!("Process:{pid} cannot mutate surface owned by Process:{owner}")),
            Self::InsufficientBacking(size, expected_size) => f.write_fmt(format_args!("Backing size mismatch for surface: size: {size} bytes, expected {expected_size} bytes")),
            Self::SurfacesExhausted => f.write_str("Cannot allocate new backing, free surfaces exhausted"),
            Self::AllocationFailed => f.write_str("Failed to allocate GPU surface"),
            Self::Unsupported => f.write_str("Unsupported surface format"),
            Self::DeviceError(msg) => f.write_fmt(format_args!("Device error: {msg}")),
            _ => todo!(),
        }
    }
}
