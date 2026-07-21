use core::fmt::{Display, Write};

#[derive(Debug)]
pub enum GpuError {
    // init errors
    DeviceRejectedFeatures,
    QueueNotAvailable,
    QueueReadyFailed,

    // resource errors
    AllocationFailed,
    ResourceCreateFailed,
    AttachBackingFailed,
    ScanoutFailed,

    // per-frame errors
    TransferFailed,
    FlushFailed,
}

impl core::fmt::Display for GpuError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}
