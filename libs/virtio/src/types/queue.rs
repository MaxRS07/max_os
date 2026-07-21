use alloc::boxed::Box;
use core::default;

use crate::QUEUE_SIZE;

#[repr(C, align(16))]
#[derive(Clone, Copy, Default, Debug)]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[repr(C, align(2))]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtqAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; QUEUE_SIZE],
}

#[repr(C, align(4))]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtqUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VirtqUsedElem; QUEUE_SIZE],
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtqUsedElem {
    pub id: u32,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct VirtQueue {
    pub desc: Box<[VirtqDesc; QUEUE_SIZE]>,
    pub avail: Box<VirtqAvail>,
    pub used: Box<VirtqUsed>,
}

impl Default for VirtQueue {
    fn default() -> Self {
        Self {
            desc: Box::new([VirtqDesc::default(); QUEUE_SIZE]),
            avail: Box::new(VirtqAvail::default()),
            used: Box::new(VirtqUsed::default()),
        }
    }
}
