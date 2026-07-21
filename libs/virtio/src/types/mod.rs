pub mod device;
pub mod queue;

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuCtrlHdr {
    pub hdr_type: u32,
    pub flags: u32,
    pub fence_id: u64, // was u32 — wrong, spec says u64
    pub ctx_id: u32,
    pub padding: u32, // was ring_idx + [u8;3] — wrong layout
}
impl VirtioGpuCtrlHdr {
    pub fn is_ok_nodata(&self) -> bool {
        self.hdr_type == 0x1100
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VirtioGpuDisplay {
    pub r: VirtioGpuRect,
    pub enabled: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VirtioGpuDisplayInfo {
    pub hdr: VirtioGpuCtrlHdr,
    pub pmodes: [VirtioGpuDisplay; 16],
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuResourceCreate2d {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    pub format: u32, // 1 = BGRA8
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuResourceAttachBacking {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    pub nr_entries: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuMemEntry {
    pub addr: u64,
    pub length: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuSetScanout {
    pub hdr: VirtioGpuCtrlHdr,
    pub r: VirtioGpuRect,
    pub scanout_id: u32,
    pub resource_id: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuTransferToHost2d {
    pub hdr: VirtioGpuCtrlHdr,
    pub r: VirtioGpuRect,
    pub offset: u64,
    pub resource_id: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuResourceFlush {
    pub hdr: VirtioGpuCtrlHdr,
    pub r: VirtioGpuRect,
    pub resource_id: u32,
    pub padding: u32,
}
