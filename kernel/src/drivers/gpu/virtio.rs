use crate::drivers::gpu::QUEUE_SIZE;

#[allow(dead_code)]
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuCtrlHdr {
    pub _type: u32,
    pub flags: u32,
    pub fence_id: u32,
    pub ctx_id: u32,
    pub ring_idx: u8,
    pub padding: [u8; 3],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VirtioGpuDisplayInfo {
    pub hdr: VirtioGpuCtrlHdr, // 24 bytes
    pub pmodes: [VirtioGpuDisplay; 16],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VirtioGpuDisplay {
    pub r: VirtioGpuRect,
    pub enabled: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
pub struct VirtqDesc {
    pub addr: u64, // physical address of buffer
    pub len: u32,
    pub flags: u16, // 0=none, 1=NEXT, 2=WRITE
    pub next: u16,  // index of next descriptor if NEXT set
}

#[repr(C, align(2))]
#[derive(Default)]
pub struct VirtqAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; QUEUE_SIZE],
}

#[repr(C, align(4))]
#[derive(Default)]
pub struct VirtqUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VirtqUsedElem; QUEUE_SIZE],
}

#[repr(C)]
#[derive(Default)]
pub struct VirtqUsedElem {
    pub id: u32,
    pub len: u32,
}

struct VirtioGpuResourceCreate2d {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32, // pick any nonzero ID, e.g. 1
    pub format: u32,      // 1 = BGRA8
    pub width: u32,
    pub height: u32,
}
struct VirtioGpuResourceAttachBacking {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    pub nr_entries: u32, // 1
}
struct VirtioGpuMemEntry {
    pub addr: u64, // physical address of your framebuffer RAM
    pub length: u32,
    pub padding: u32,
}
struct VirtioGpuSetScanout {
    pub hdr: VirtioGpuCtrlHdr,
    pub r: VirtioGpuRect, // {0, 0, width, height}
    pub scanout_id: u32,  // 0
    pub resource_id: u32, // 1
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct GpuCtrlHdr {
    pub hdr_type: u32,
    pub flags: u32,
    pub fence_id: u64,
    pub ctx_id: u32,
    pub padding: u32,
}
