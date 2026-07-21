// Offsets
pub const CONFIG_BASE: u32 = 0x100;

// Queues
pub const VIRTIO_INPUT_QUEUE_EVENT: u32 = 0;
pub const VIRTIO_INPUT_QUEUE_STATUS: u32 = 1;

// Select values
pub const VIRTIO_INPUT_CFG_UNSET: u8 = 0x00;
pub const VIRTIO_INPUT_CFG_ID_NAME: u8 = 0x01;
pub const VIRTIO_INPUT_CFG_ID_SERIAL: u8 = 0x02;
pub const VIRTIO_INPUT_CFG_ID_DEVIDS: u8 = 0x03;
pub const VIRTIO_INPUT_CFG_PROP_BITS: u8 = 0x10;
pub const VIRTIO_INPUT_CFG_EV_BITS: u8 = 0x11;
pub const VIRTIO_INPUT_CFG_ABS_INFO: u8 = 0x12;

// Subsel values
pub const EV_SYN: u8 = 0x00;
pub const EV_KEY: u8 = 0x01; // keys/buttons
pub const EV_REL: u8 = 0x02; // relative axes (mouse movement)
pub const EV_ABS: u8 = 0x03; // absolute axes (tablet/touchscreen)
pub const EV_MSC: u8 = 0x04;
pub const EV_SW: u8 = 0x05; // switches
pub const EV_LED: u8 = 0x11;
pub const EV_SND: u8 = 0x12;
pub const EV_REP: u8 = 0x14;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VirtioInputAbsinfo {
    pub min: u32,
    pub max: u32,
    pub fuzz: u32,
    pub flat: u32,
    pub res: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VirtioInputDevids {
    pub bustype: u16,
    pub vendor: u16,
    pub product: u16,
    pub version: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union U {
    pub bitmap: [u8; 128],
    pub abs: VirtioInputAbsinfo,
    pub ids: VirtioInputDevids,
}
impl core::fmt::Debug for U {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("U")
            .field("bitmap", unsafe { &self.bitmap })
            .field("abs", unsafe { &self.abs })
            .field("ids", unsafe { &self.ids })
            .finish()
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VirtioInputConfig {
    pub select: u8,
    pub subsel: u8,
    pub size: u8,
    pub reserved: [u8; 5],
    pub u: U,
}

// Events

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VirtioInputEvent {
    pub type_: u16,
    pub code: u16,
    pub value: u32,
}
