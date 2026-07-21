#![no_std]

pub mod init;
pub mod queue;
pub mod send;
pub mod types;

// TODO: Revamp for complex gpu operations
pub const QUEUE_SIZE: usize = 0x10;

// VirtIO Vendor + Device IDs
pub const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
pub const VIRTIO_DEV_NETWORK: u16 = 0x1040;
pub const VIRTIO_DEV_BLOCK: u16 = 0x1041;
pub const VIRTIO_DEV_GPU: u16 = 0x1050;
pub const VIRTIO_DEV_INPUT: u16 = 0x1052;
pub const VIRTIO_DEV_SOUND: u16 = 0x1059;

// VirtIO MMIO Device IDs (for non-PCI virtio)
pub const VIRTIO_MMIO_DEV_BLOCK: u32 = 2;
pub const VIRTIO_MMIO_DEV_GPU: u32 = 16;
pub const VIRTIO_MMIO_DEV_INPUT: u32 = 18;
pub const VIRTIO_MMIO_DEV_SOUND: u32 = 25;

// VirtIO MMIO Register Offsets
pub const VIRTIO_MMIO_MAGIC: u32 = 0x000;
pub const VIRTIO_MMIO_VERSION: u32 = 0x004;
pub const VIRTIO_MMIO_DEVICE_ID: u32 = 0x010;
pub const VIRTIO_MMIO_VENDOR_ID: u32 = 0x00C;

pub const VIRTIO_MMIO_DEV_FEATURES: u32 = 0x010;
pub const VIRTIO_MMIO_DEV_FEATURES_SEL: u32 = 0x014;

pub const VIRTIO_MMIO_DRV_FEATURES: u32 = 0x020;
pub const VIRTIO_MMIO_DRV_FEATURES_SEL: u32 = 0x024;

pub const VIRTIO_MMIO_QUEUE_SEL: u32 = 0x030;
pub const VIRTIO_MMIO_QUEUE_MAX: u32 = 0x034;
pub const VIRTIO_MMIO_QUEUE_NUM: u32 = 0x038;
pub const VIRTIO_MMIO_QUEUE_READY: u32 = 0x044;
pub const VIRTIO_MMIO_QUEUE_NOTIFY: u32 = 0x050;

pub const VIRTIO_MMIO_INT_STATUS: u32 = 0x060;
pub const VIRTIO_MMIO_INT_ACK: u32 = 0x064;

pub const VIRTIO_MMIO_STATUS: u32 = 0x070;

pub const VIRTIO_MMIO_QUEUE_DESC: u32 = 0x080;
pub const VIRTIO_MMIO_QUEUE_DRIVER: u32 = 0x090;
pub const VIRTIO_MMIO_QUEUE_DEVICE: u32 = 0x0A0;

pub const VIRTIO_MMIO_MAGIC_VALUE: u32 = 0x74726976; // "virt"

extern crate alloc;
