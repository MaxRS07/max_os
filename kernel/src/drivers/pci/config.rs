// kernel/src/devices/pci/config.rs

// PCI Configuration Space Offsets
pub const PCI_CFG_VENDOR_ID: u16 = 0x00;
pub const PCI_CFG_DEVICE_ID: u16 = 0x02;
pub const PCI_CFG_COMMAND: u16 = 0x04;
pub const PCI_CFG_STATUS: u16 = 0x06;
pub const PCI_CFG_REVISION: u16 = 0x08;
pub const PCI_CFG_PROG_IF: u16 = 0x09;
pub const PCI_CFG_SUBCLASS: u16 = 0x0A;
pub const PCI_CFG_CLASS: u16 = 0x0B;
pub const PCI_CFG_CACHE_LINE: u16 = 0x0C;
pub const PCI_CFG_LATENCY: u16 = 0x0D;
pub const PCI_CFG_HEADER_TYPE: u16 = 0x0E;
pub const PCI_CFG_BIST: u16 = 0x0F;
pub const PCI_CFG_BAR0: u16 = 0x10;
pub const PCI_CFG_BAR1: u16 = 0x14;
pub const PCI_CFG_BAR2: u16 = 0x18;
pub const PCI_CFG_BAR3: u16 = 0x1C;
pub const PCI_CFG_BAR4: u16 = 0x20;
pub const PCI_CFG_BAR5: u16 = 0x24;
pub const PCI_CFG_SUBSYS_VID: u16 = 0x2C;
pub const PCI_CFG_SUBSYS_ID: u16 = 0x2E;
pub const PCI_CFG_CAP_PTR: u16 = 0x34;
pub const PCI_CFG_INT_LINE: u16 = 0x3C;
pub const PCI_CFG_INT_PIN: u16 = 0x3D;

// PCI Command Register Bits (PCI_CFG_COMMAND)
pub const PCI_CMD_IO_SPACE: u16 = 1 << 0;
pub const PCI_CMD_MEM_SPACE: u16 = 1 << 1; // enable memory BARs
pub const PCI_CMD_BUS_MASTER: u16 = 1 << 2; // enable DMA
pub const PCI_CMD_INT_DISABLE: u16 = 1 << 10;

// PCI Header Types
pub const PCI_HEADER_NORMAL: u8 = 0x00;
pub const PCI_HEADER_BRIDGE: u8 = 0x01;
pub const PCI_HEADER_CARDBUS: u8 = 0x02;
pub const PCI_HEADER_MULTI_FN: u8 = 0x80; // bit 7 = multifunction device

// PCI BAR Flags (bottom bits of BAR register)
pub const PCI_BAR_IO: u32 = 1 << 0; // 1 = I/O, 0 = memory
pub const PCI_BAR_TYPE_32: u32 = 0x0 << 1;
pub const PCI_BAR_TYPE_64: u32 = 0x2 << 1;
pub const PCI_BAR_PREFETCH: u32 = 1 << 3;
pub const PCI_BAR_ADDR_MASK: u32 = 0xFFFFFFF0;

// Sentinel values
pub const PCI_VENDOR_NONE: u16 = 0xFFFF; // empty slot

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
pub const VIRTIO_MMIO_MAGIC: usize = 0x000;
pub const VIRTIO_MMIO_VERSION: usize = 0x004;
pub const VIRTIO_MMIO_DEVICE_ID: u32 = 0x010;
pub const VIRTIO_MMIO_VENDOR_ID: usize = 0x00C;
pub const VIRTIO_MMIO_DEV_FEATURES: usize = 0x010;
pub const VIRTIO_MMIO_DRV_FEATURES: usize = 0x020;
pub const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030;
pub const VIRTIO_MMIO_QUEUE_MAX: usize = 0x034;
pub const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038;
pub const VIRTIO_MMIO_QUEUE_READY: usize = 0x044;
pub const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050;
pub const VIRTIO_MMIO_INT_STATUS: usize = 0x060;
pub const VIRTIO_MMIO_INT_ACK: usize = 0x064;
pub const VIRTIO_MMIO_STATUS: usize = 0x070;
pub const VIRTIO_MMIO_QUEUE_DESC: usize = 0x080;
pub const VIRTIO_MMIO_QUEUE_DRIVER: usize = 0x090;
pub const VIRTIO_MMIO_QUEUE_DEVICE: usize = 0x0A0;
pub const VIRTIO_MMIO_MAGIC_VALUE: u32 = 0x74726976; // "virt"
