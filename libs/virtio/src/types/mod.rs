use core::default;

pub mod device;
pub mod queue;

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuCtrlHdr {
    pub hdr_type: u32,
    pub flags: u32,
    pub fence_id: u64,
    pub ctx_id: u32,
    pub _padding: u32,
}
impl VirtioGpuCtrlHdr {
    pub fn is_ok_nodata(&self) -> bool {
        self.hdr_type == 0x1100
    }
    pub fn is_ok_info(&self) -> bool {
        self.hdr_type == VIRTIO_GPU_RESP_OK_DISPLAY_INFO
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
pub struct VirtioGpuResourceCreate2D {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    pub format: u32, // 1 = BGRA8
    pub width: u32,
    pub height: u32,
}
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuResourceCreate3D {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    pub target: u32,
    pub format: u32,
    pub bind: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub array_size: u32,
    pub last_level: u32,
    pub nr_samples: u32,
    pub flags: u32,
    _padding: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VirtioGpuResourceUnref {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    _padding: u32,
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
    pub _padding: u32,
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
    pub _padding: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtioGpuResourceFlush {
    pub hdr: VirtioGpuCtrlHdr,
    pub r: VirtioGpuRect,
    pub resource_id: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VirtioGpuCursorPos {
    pub scanout_id: u32,
    pub x: u32,
    pub y: u32,
    _padding: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VirtioGpuUpdateCursor {
    pub hdr: VirtioGpuCtrlHdr,
    pub pos: VirtioGpuCursorPos,
    pub resource_id: u32,
    pub hot_x: u32,
    pub hot_y: u32,
    _padding: u32,
}

pub const VIRTIO_GPU_BLOB_MEM_GUEST: u32 = 0x0001;
pub const VIRTIO_GPU_BLOB_MEM_HOST3D: u32 = 0x0002;
pub const VIRTIO_GPU_BLOB_MEM_HOST3D_GUEST: u32 = 0x0003;
pub const VIRTIO_GPU_BLOB_FLAG_USE_MAPPABLE: u32 = 0x0001;
pub const VIRTIO_GPU_BLOB_FLAG_USE_SHAREABLE: u32 = 0x0002;
pub const VIRTIO_GPU_BLOB_FLAG_USE_CROSS_DEVICE: u32 = 0x0004;

pub struct VirtioGpuResourceCreateBlob {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    /// blob_mem can only be `VIRTIO_GPU_BLOB_MEM_HOST3D` and `VIRTIO_GPU_BLOB_MEM_HOST3D_GUEST` if `VIRTIO_GPU_F_VIRGL` is supported.
    ///  `VIRTIO_GPU_BLOB_MEM_GUEST` is valid regardless whether `VIRTIO_GPU_F_VIRGL` is supported or not.
    ///
    /// For `VIRTIO_GPU_BLOB_MEM_HOST3D` and `VIRTIO_GPU_BLOB_MEM_HOST3D_GUEST`, the virtio-gpu resource MUST be created from the
    /// rendering context local object identified by the [`blob_id`]. The actual allocation is done via `VIRTIO_GPU_CMD_SUBMIT_3D`.
    pub blob_mem: u32,
    /// The driver MUST inform the device if the blob resource is used for memory access, sharing between driver instances and/or sharing with other devices.
    /// This is done via the `blob_flags` field.
    pub blob_flags: u32,
    /// specifies the number of [`VirtioGpuMemEntry`] supplied to back the resource.
    /// Basically the number of extents describing contiguous blocks of memory for this BLOB
    pub nr_entries: u32,
    /// identifier, seperate from GPU resources
    pub blob_id: u64,
    // size in bytes
    pub size: u64,
}
