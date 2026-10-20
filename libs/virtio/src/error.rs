use core::fmt::Display;

use alloc::{format, string::String};

use crate::{VIRTIO_MMIO_MAGIC_VALUE, types::device};

#[derive(Clone, Debug)]
pub enum VirtioError {
    /// Contains the read magic value
    BadMagic(u32),
    /// `(actual_id, expected_id)`
    BadId(u32, u32),
    TimeoutError,
    ErrorResponse(u32),
    /// `(device_type: &'static str, unsupported_feature: &'static str)`
    NotImplemented(&'static str, &'static str),
    /// Features rejected by device
    FeaturesRejected,
    QueueInitFailed(u32),
    QueueUnavailable(u32),
    /// misc device specific errors
    DeviceError(String),
}

impl Display for VirtioError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadMagic(magic) => f.write_fmt(format_args!(
                "Incorrect magic value for VirtioDevice expected {:#x}, found {:#x}",
                VIRTIO_MMIO_MAGIC_VALUE, magic
            )),
            Self::BadId(real, expected) => f.write_fmt(format_args!(
                "Found device with id: {real}, expected: {expected}"
            )),
            Self::TimeoutError => f.write_str("Command timed out"),
            Self::ErrorResponse(resp) => f.write_str("Error code returned from device "),
            Self::NotImplemented(device, feature) => {
                f.write_fmt(format_args!("Feature: {feature} not supported on {device}"))
            }
            Self::FeaturesRejected => f.write_str("Device rejected features"),
            Self::DeviceError(msg) => f.write_str(msg),
            Self::QueueInitFailed(id) => {
                f.write_fmt(format_args!("Failed to initialize virtqueue with id: {id}"))
            }
            Self::QueueUnavailable(id) => f.write_fmt(format_args!("Queue id: {id} not available")),
        }
    }
}
