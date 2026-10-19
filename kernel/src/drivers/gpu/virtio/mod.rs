use core::{
    alloc::Layout,
    convert::identity,
    net::AddrParseError,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    vec::{self, Vec},
};
use graphics::{bgra::Brga, device::GpuDevice, error::GpuError};
use log::{debug, error, warn};
use mem::error::MemoryError;
use sdt::fdt::FDT;
use virtio::{
    ALL_FEATURES,
    error::VirtioError,
    init::{init_state, signal_driver_ok},
    queue::init_virtqueue,
    types::{
        VirtioGpuCtrlHdr, VirtioGpuDisplayInfo, VirtioGpuMemEntry, VirtioGpuRect,
        VirtioGpuResourceAttachBacking, VirtioGpuResourceCreate2d, VirtioGpuResourceFlush,
        VirtioGpuResourceUnref, VirtioGpuSetScanout, VirtioGpuTransferToHost2d,
        device::verify_virtio_magic,
        queue::{DescBuf, VirtQueue},
    },
};

use crate::drivers::gpu::{
    error::GpuError,
    virtio::cmd::{
        VIRTIO_GPU_CMD_GET_DISPLAY_INFO, VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
        VIRTIO_GPU_CMD_RESOURCE_CREATE_2D, VIRTIO_GPU_CMD_RESOURCE_FLUSH,
        VIRTIO_GPU_CMD_RESOURCE_UNREF, VIRTIO_GPU_CMD_SET_SCANOUT,
        VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D, VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM,
    },
};

// Mostly adapted from specification here: https://docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html#x1-3960007
pub mod cmd;

// limit to 1080p for now
const MAX_RESOLUTION: (usize, usize) = (1920, 1080);
const MAX_BUFFER_SIZE: usize = MAX_RESOLUTION.0 * MAX_RESOLUTION.1;

enum VirtioGpuResource {
    TwoD { width: u32, height: u32 },
    ThreeD {},
    Blob(),
}
/// VirtIO GPU Device
#[derive(Default)]
pub struct VirtioGpu {
    mmio_addr: usize,
    width: u32,
    height: u32,
    id_allocator: AtomicU32,
    controlq: VirtQueue,
    cursorq: VirtQueue,
    resources: BTreeMap<u32, VirtioGpuResource>,
}

impl VirtioGpu {
    pub fn from_mmio(fdt: &FDT, mmio_idx: usize) -> Option<Self> {
        let mmio_addr = fdt.virtio_mmio[mmio_idx].base_address;
        if let Err(error) = verify_virtio_magic(mmio_addr) {
            warn!("{}", error);
            return None;
        }
        if let Err(error) = init_state(mmio_addr, ALL_FEATURES) {
            warn!("Failed to initialize GPU state: {}", error);
            return None;
        }

        let controlq = VirtQueue::default();
        let cursorq = VirtQueue::default();

        let mut virtio_gpu = VirtioGpu {
            mmio_addr,
            controlq,
            cursorq,
            width: 0,
            height: 0,
            id_allocator: AtomicU32::new(0),
            resources: BTreeMap::new(),
        };

        if let Err(error) = init_virtqueue(&mut virtio_gpu.controlq, mmio_addr, 0) {
            error!("Failed to initialize virtqueue: {}", error);
            return None;
        }
        debug!("initialized virtqueue");
        signal_driver_ok(mmio_addr);
        let info = virtio_gpu.get_display_info();
        let rect = info.pmodes[0].r;

        if let Err(error) = virtio_gpu.initialize_framebuffer(rect.width, rect.height) {
            error!("Failed to initialize framebuffer for VirtIO GPU: {}", error);
        }
        debug!("initialized framebuffer");
        virtio_gpu.scanout(1, rect);
        if let Err(error) = virtio_gpu.flush_resources() {
            error!("Failed to flush VirtIO GPU: {}", error)
        }
        debug!("flushed resources");

        Some(virtio_gpu)
    }

    fn get_display_info(&mut self) -> VirtioGpuDisplayInfo {
        let request = VirtioGpuCtrlHdr {
            hdr_type: VIRTIO_GPU_CMD_GET_DISPLAY_INFO,
            ..Default::default()
        };
        let mut response = VirtioGpuDisplayInfo::default();

        self.controlq
            .push_desc(self.mmio_addr, &request, &mut response);

        response
    }

    fn create_resource(&mut self, width: u32, height: u32) -> Result<u32, VirtioError> {
        let resource_id = self.id_allocator.fetch_add(1, Ordering::Acquire);
        let mut request = VirtioGpuResourceCreate2d::default();
        request.hdr.hdr_type = VIRTIO_GPU_CMD_RESOURCE_CREATE_2D;
        request.resource_id = resource_id; // fb uuid, TODO: add id utility
        request.format = VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM;
        request.width = width;
        request.height = height;

        let mut response = VirtioGpuCtrlHdr::default();

        self.controlq.push_desc_timeout(
            self.mmio_addr,
            &request,
            &mut response,
            Duration::from_secs(1),
        )?;

        if !response.is_ok_nodata() {
            return Err(VirtioError::ErrorResponse(response.hdr_type));
        }

        Ok(resource_id)
    }
    /* Cursor operation */
    pub fn create_cursor_resource(&mut self) -> Result<u32, VirtioError> {
        let cursor_id = self.create_resource(64, 64)?;
        self.attach_backing(cursor_id)?;
        Ok(cursor_id)
    }
    /// Removes a resource from this gpu, deallocating the attached backing
    fn destroy_resource(&mut self, resource_id: u32) -> Result<(), VirtioError> {
        let mut request = VirtioGpuResourceUnref::default();
        request.hdr.hdr_type = VIRTIO_GPU_CMD_RESOURCE_UNREF;
        request.resource_id = resource_id;

        let mut response = VirtioGpuCtrlHdr::default();

        self.controlq.push_desc_timeout(
            self.mmio_addr,
            &request,
            &mut response,
            Duration::from_secs(1),
        )?;

        Ok(())
    }
    /// Allocates memory for a gpu resource given a device id
    fn attach_backing(&mut self, resource_id: u32) -> VirtioGpuCtrlHdr {
        let mut request = Box::new(VirtioGpuResourceAttachBacking {
            hdr: VirtioGpuCtrlHdr {
                hdr_type: VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
                ..Default::default()
            },
            resource_id,
            nr_entries: 1,
        });
        self.get_resource(resource_id);
        let buffer= Box::new(x)
        let mut entry = Box::new(VirtioGpuMemEntry {
            addr: fb_addr,
            length: fb_size as u32,
            padding: 0,
        });
        let mut response = Box::new(VirtioGpuCtrlHdr::default());

        let req_addr = &mut *request as *mut VirtioGpuResourceAttachBacking as usize;
        let entry_addr = &mut *entry as *mut VirtioGpuMemEntry as usize;
        let address = &mut *response as *mut VirtioGpuCtrlHdr as usize;

        let last = self.controlq.push_desc_raw(
            self.mmio_addr,
            &[
                DescBuf {
                    addr: req_addr as u64,
                    len: size_of::<VirtioGpuResourceAttachBacking>() as u32,
                    flags: 0x0,
                },
                DescBuf {
                    addr: entry_addr as u64,
                    len: size_of::<VirtioGpuMemEntry>() as u32,
                    flags: 0x0,
                },
                DescBuf {
                    addr: address as u64,
                    len: size_of::<VirtioGpuCtrlHdr>() as u32,
                    flags: 0x2,
                },
            ],
            0,
        );
        self.controlq.wait_used(last);

        *response
    }

    fn scanout(&mut self, resource_id: u32, rect: VirtioGpuRect) -> Result<(), VirtioError> {
        let request = VirtioGpuSetScanout {
            hdr: VirtioGpuCtrlHdr {
                hdr_type: VIRTIO_GPU_CMD_SET_SCANOUT,
                ..Default::default()
            },
            r: rect,
            scanout_id: 0,
            resource_id,
        };
        let mut response = VirtioGpuCtrlHdr::default();
        self.controlq.push_desc_timeout(
            self.mmio_addr,
            &request,
            &mut response,
            Duration::from_secs(1),
        )
    }

    fn flush_resources(&mut self) -> Result<(), GpuError> {
        // upload fb memory to GPU resource
        let transfer_req = VirtioGpuTransferToHost2d {
            hdr: VirtioGpuCtrlHdr {
                hdr_type: VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D,
                ..Default::default()
            },
            r: VirtioGpuRect {
                x: 0,
                y: 0,
                width: self.width,
                height: self.height,
            },
            offset: 0,
            resource_id: 1,
            padding: 0,
        };

        let mut response = VirtioGpuCtrlHdr::default();
        self.controlq
            .push_desc(self.mmio_addr, &transfer_req, &mut response);

        // flush to scanout
        let flush_req = VirtioGpuResourceFlush {
            hdr: VirtioGpuCtrlHdr {
                hdr_type: VIRTIO_GPU_CMD_RESOURCE_FLUSH,
                ..Default::default()
            },
            r: VirtioGpuRect {
                x: 0,
                y: 0,
                width: self.width,
                height: self.height,
            },
            resource_id: 1,
            padding: 0,
        };
        self.controlq
            .push_desc(self.mmio_addr, &flush_req, &mut response);
        Ok(())
    }
}

impl GpuDevice for VirtioGpu {
    /// Creates a memory backed resource for the GPU, returns [`Ok`] containing the resourece ID on success
    fn create_resource(&mut self, width: u32, height: u32) -> Result<u32, GpuError> {
        self.create_resource(width, height)
            .map_err(|err| GpuError::ResourceCreateFailed)
    }
    fn destroy_resource(&mut self, resource_id: u32) -> Result<(), GpuError> {
        self.destroy_resource(resource_id)
            .map_err(|_| GpuError::ResourceDestroyFailed)
    }
    /// query resources by resource id
    fn get_resource(&mut self, resource_id: u32) -> Option<VirtioGpuResource> {
        self.resources.get(&resource_id)
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn framebuffer(&mut self) -> &mut [graphics::bgra::Brga] {
        self.framebuffer.as_mut()
    }

    fn flush(&mut self) -> Result<(), GpuError> {
        self.flush_resources()
    }

    fn flip(&mut self) -> Result<(), GpuError> {
        todo!()
    }
}
