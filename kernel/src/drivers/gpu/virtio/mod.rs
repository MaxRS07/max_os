use core::{alloc::Layout, net::AddrParseError};

use alloc::{
    boxed::Box,
    vec::{self, Vec},
};
use graphics::{gpu::GpuDevice, types::Brga};
use log::{error, info, warn};
use mmio::{mmio_read, mmio_write};
use sdt::fdt::FDT;
use virtio::{
    QUEUE_SIZE, VIRTIO_MMIO_MAGIC, VIRTIO_MMIO_MAGIC_VALUE, VIRTIO_MMIO_QUEUE_DESC,
    VIRTIO_MMIO_QUEUE_DEVICE, VIRTIO_MMIO_QUEUE_DRIVER, VIRTIO_MMIO_QUEUE_MAX,
    VIRTIO_MMIO_QUEUE_NUM, VIRTIO_MMIO_QUEUE_SEL,
    init::init_state,
    queue::init_virtqueue,
    send::{DescBuf, send_command, send_command_raw},
    types::{
        VirtioGpuCtrlHdr, VirtioGpuDisplayInfo, VirtioGpuMemEntry, VirtioGpuRect,
        VirtioGpuResourceAttachBacking, VirtioGpuResourceCreate2d, VirtioGpuResourceFlush,
        VirtioGpuSetScanout, VirtioGpuTransferToHost2d,
        queue::{VirtQueue, VirtqAvail, VirtqDesc, VirtqUsed},
    },
};

use crate::{
    drivers::gpu::{
        error::GpuError,
        virtio::cmd::{
            VIRTIO_GPU_CMD_GET_DISPLAY_INFO, VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
            VIRTIO_GPU_CMD_RESOURCE_CREATE_2D, VIRTIO_GPU_CMD_RESOURCE_FLUSH,
            VIRTIO_GPU_CMD_SET_SCANOUT, VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D,
            VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM,
        },
    },
    mm::error::MemoryError,
};

// Mostly adapted from specification here: https://docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html#x1-3960007
pub mod cmd;

// limit to 1080p for now
const MAX_RESOLUTION: (usize, usize) = (1920, 1080);
const MAX_BUFFER_SIZE: usize = MAX_RESOLUTION.0 * MAX_RESOLUTION.1;

/// VirtIO GPU Device
#[derive(Default)]
pub struct VirtioGpu {
    mmio_addr: usize,
    framebuffer: Box<[graphics::types::Brga]>,
    width: u32,
    height: u32,
    resource_id: u32,
    queue: VirtQueue,
}

impl VirtioGpu {
    pub fn from_mmio(fdt: &FDT, mmio_idx: usize) -> Option<Self> {
        let mmio_addr = fdt.virtio_mmio[mmio_idx].base_address;
        let magic = mmio::mmio_read::<u32>(mmio_addr, VIRTIO_MMIO_MAGIC);
        if magic != VIRTIO_MMIO_MAGIC_VALUE {
            return None;
        }
        if let Err(error) = init_state(mmio_addr) {
            warn!("Failed to initialize GPU state: {}", error);
            return None;
        }

        let queue = VirtQueue::default();

        let mut virtio_gpu = VirtioGpu {
            mmio_addr,
            queue,
            framebuffer: Box::new([]), // placeholder, replaced in initialize_framebuffer
            width: 0,
            height: 0,
            resource_id: 0,
        };

        if let Err(error) = init_virtqueue(&mut virtio_gpu.queue, mmio_addr, 0) {
            error!("Failed to initialize virtqueue: {}", error);
            return None;
        }
        // signal DRIVER_OK
        mmio_write::<u32>(mmio_addr, 0x70, 15);

        let info = virtio_gpu.get_display_info();
        let rect = info.pmodes[0].r;

        if let Err(error) = virtio_gpu.initialize_framebuffer(rect.width, rect.height) {
            error!("Failed to initialize framebuffer for VirtIO GPU: {}", error);
        }
        virtio_gpu.scanout(1, rect);
        if let Err(error) = virtio_gpu.flush_resources() {
            error!("Failed to flush VirtIO GPU: {}", error)
        }

        Some(virtio_gpu)
    }

    fn get_display_info(&mut self) -> VirtioGpuDisplayInfo {
        let request = VirtioGpuCtrlHdr {
            hdr_type: VIRTIO_GPU_CMD_GET_DISPLAY_INFO,
            ..Default::default()
        };
        let mut response = VirtioGpuDisplayInfo::default();

        send_command(self.mmio_addr, &mut self.queue, &request, &mut response);

        response
    }

    fn create_resource(&mut self, resource_id: u32, width: u32, height: u32) -> VirtioGpuCtrlHdr {
        let mut request = VirtioGpuResourceCreate2d::default();
        request.hdr.hdr_type = VIRTIO_GPU_CMD_RESOURCE_CREATE_2D;
        request.resource_id = resource_id; // fb uuid, TODO: add id utility
        request.format = VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM;
        request.width = width;
        request.height = height;

        let mut response = VirtioGpuCtrlHdr::default();

        send_command(self.mmio_addr, &mut self.queue, &request, &mut response);

        response
    }

    fn attach_backing(&mut self, resource_id: u32) -> VirtioGpuCtrlHdr {
        let mut request = Box::new(VirtioGpuResourceAttachBacking {
            hdr: VirtioGpuCtrlHdr {
                hdr_type: VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
                ..Default::default()
            },
            resource_id,
            nr_entries: 1,
        });
        let fb_addr = &self.framebuffer[0] as *const Brga as u64;
        let fb_size = self.framebuffer.len() * size_of::<Brga>();
        let mut entry = Box::new(VirtioGpuMemEntry {
            addr: fb_addr,
            length: fb_size as u32,
            padding: 0,
        });
        let mut response = Box::new(VirtioGpuCtrlHdr::default());

        let req_addr = &mut *request as *mut VirtioGpuResourceAttachBacking as usize;
        let entry_addr = &mut *entry as *mut VirtioGpuMemEntry as usize;
        let address = &mut *response as *mut VirtioGpuCtrlHdr as usize;

        send_command_raw(
            self.mmio_addr,
            &mut self.queue,
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
        );

        response.as_ref().clone()
    }

    fn scanout(&mut self, resource_id: u32, rect: VirtioGpuRect) {
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
        send_command(self.mmio_addr, &mut self.queue, &request, &mut response);
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
        send_command(
            self.mmio_addr,
            &mut self.queue,
            &transfer_req,
            &mut response,
        );

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
        send_command(self.mmio_addr, &mut self.queue, &flush_req, &mut response);
        Ok(())
    }

    fn initialize_framebuffer(&mut self, width: u32, height: u32) -> Result<(), MemoryError> {
        let id = 1;
        if self.create_resource(id, width, height).is_ok_nodata() {
            let size = (width * height) as usize;

            // FIX: Allocates directly on the heap without cloning on the stack
            let framebuffer: Box<[Brga]> = core::iter::repeat_with(Brga::default)
                .take(size)
                .collect::<Box<[Brga]>>();

            self.framebuffer = framebuffer;

            let response = self.attach_backing(id);
            if response.is_ok_nodata() {
                return Ok(());
            }
        }
        Err(MemoryError::AccessViolation(""))
    }
}

impl GpuDevice for VirtioGpu {
    type Error = GpuError;

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn framebuffer(&mut self) -> &mut [graphics::types::Brga] {
        self.framebuffer.as_mut()
    }

    fn flush(&mut self) -> Result<(), GpuError> {
        self.flush_resources()
    }
}
