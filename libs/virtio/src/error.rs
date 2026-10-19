use core::fmt::Display;

use alloc::string::String;

use crate::VIRTIO_MMIO_MAGIC_VALUE;

#[derive(Clone, Copy, Debug)]
pub enum VirtioError {
    BadMagic(u32),
    TimeoutError,
    ErrorResponse(u32),
}

impl Display for VirtioError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadMagic(magic) => f.write_fmt(format_args!(
                "Incorrect magic value for VirtioDevice expected {:#x}, found {:#x}",
                VIRTIO_MMIO_MAGIC_VALUE, magic
            )),
            Self::TimeoutError => f.write_str("Command timed out"),
            Self::ErrorResponse(resp) => f.write_str("Error code returned from device "),
        }
    }
}
