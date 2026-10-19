use core::{
    fmt::{Display, Write},
    todo,
};

use crate::error::GpuError::DeviceRejectedFeatures;

#[derive(Debug)]
pub enum GpuError {
    // init errors
    DeviceRejectedFeatures,
    QueueNotAvailable,
    QueueReadyFailed,

    // resource errors
    AllocationFailed,
    ResourceCreateFailed,
    ResourceDestroyFailed,
    AttachBackingFailed,
    ScanoutFailed,

    // per-frame errors
    TransferFailed,
    FlushFailed,
}

impl core::fmt::Display for GpuError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DeviceRejectedFeatures => f.write_str("Device rejected feature bits"),
            _ => todo!(),
        }
    }
}
