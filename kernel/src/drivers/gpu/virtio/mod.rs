use core::{
    alloc::Layout,
    convert::identity,
    net::AddrParseError,
    panicking::panic,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    vec::{self, Vec},
};
use fdt::tree::FDT;
use graphics::{bgra::Brga, device::GpuDevice, error::GpuError};
use log::{debug, error, warn};
use mem::error::MemoryError;
use virtio::{
    ALL_FEATURES,
    error::VirtioError,
    init::{init_state, signal_driver_ok},
    queue::init_virtqueue,
    types::{
        VirtioGpuCtrlHdr, VirtioGpuDisplayInfo, VirtioGpuMemEntry, VirtioGpuRect,
        VirtioGpuResourceAttachBacking, VirtioGpuResourceCreate2D, VirtioGpuResourceCreate3D,
        VirtioGpuResourceFlush, VirtioGpuResourceUnref, VirtioGpuSetScanout,
        VirtioGpuTransferToHost2d,
        device::{VirtioDevice, verify_virtio_magic},
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
    TwoD {
        width: u32,
        height: u32,
        format: u32,
        addr: usize,
        size: usize,
    },
    ThreeD {
        width: u32,
        height: u32,
        depth: u32,
        format: u32,
    },
}
/// VirtIO GPU Device
pub struct VirtioGpu {
    device: VirtioDevice,
    width: u32,
    height: u32,
    id_allocator: AtomicU32,
    controlq: VirtQueue,
    cursorq: VirtQueue,
    resources: BTreeMap<u32, VirtioGpuResource>,
}

impl VirtioGpu {
    pub fn new(fdt: &FDT, mmio_idx: usize) -> Result<Self, VirtioError> {
        let gpu_device = VirtioDevice::probe(fdt, mmio_idx, 16)?;
        let mut virtio_gpu = VirtioGpu {
            device: gpu_device,
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
        let info = virtio_gpu.get_display_info()?;
        let rect = info.pmodes[0].r;

        debug!("initialized framebuffer");
        virtio_gpu.scanout(1, rect);
        if let Err(error) = virtio_gpu.flush_resources() {
            error!("Failed to flush VirtIO GPU: {}", error)
        }
        debug!("flushed resources");

        Some(virtio_gpu)
    }

    fn get_display_info(&mut self) -> Result<VirtioGpuDisplayInfo, VirtioError> {
        let request = VirtioGpuCtrlHdr {
            hdr_type: VIRTIO_GPU_CMD_GET_DISPLAY_INFO,
            ..Default::default()
        };
        let mut response = VirtioGpuDisplayInfo::default();

        self.controlq.push_desc_timeout(
            self.mmio_addr,
            &request,
            &mut response,
            Duration::from_secs(1),
        );
        if !response.hdr.is_ok_nodata() {
            return Err(VirtioError::ErrorResponse(response.hdr.hdr_type));
        }
        Ok(response)
    }

    fn register_resource(&mut self, width: u32, height: u32) -> Result<u32, VirtioError> {
        let resource_id = self.create_resource2d(width, height)?;
        self.attach_backing(resource_id)?;
        Ok(resource_id)
    }

    fn create_resource2d(&mut self, width: u32, height: u32) -> Result<u32, VirtioError> {
        let resource_id = self.id_allocator.fetch_add(1, Ordering::Acquire);
        let mut request = VirtioGpuResourceCreate2D::default();
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

        self.resources.insert(
            resource_id,
            VirtioGpuResource::TwoD {
                width,
                height,
                format: VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM,
                addr: 0,
                size: 0,
            },
        );

        Ok(resource_id)
    }
    // fn create_resource3d(
    //     &mut self,
    //     width: u32,
    //     height: u32,
    //     depth: u32,
    // ) -> Result<u32, VirtioError> {
    //     // let resource_id = self.id_allocator.fetch_add(1, Ordering::Acquire);
    //     // let mut request = VirtioGpuResourceCreate3D::default();
    //     // request.resource_id = resource_id;
    //     // request.format = VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM;
    //     // request.width = width;
    //     // request.height = height;
    //     // request.depth = depth;
    // }
    /* Cursor operation */
    pub fn create_cursor_resource(&mut self) -> Result<u32, VirtioError> {
        let cursor_id = self.create_resource2d(64, 64)?;
        self.attach_backing(cursor_id)?;
        Ok(cursor_id)
    }
    /// Removes a resource from this gpu, deallocating the attached backing. Returns [`Ok`] on success conraining the size of the freed backing in bytes.
    fn destroy_resource(&mut self, resource_id: u32) -> Result<usize, VirtioError> {
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

        match self.get_mut_resource(resource_id)? {
            VirtioGpuResource::TwoD { addr, size, .. } => unsafe {
                let ptr = *addr as *mut u8;
                let buffer = Vec::from_raw_parts(ptr, *size, *size);
                // free the memory
                drop(buffer);

                self.resources.remove(&resource_id);
                return Ok(*size);
            },
            _ => todo!(),
        }
    }
    /// Allocates memory for a gpu resource given a resource id. Automatically detects required size and shape
    fn attach_backing(&mut self, resource_id: u32) -> Result<VirtioGpuCtrlHdr, VirtioError> {
        let mut request = Box::new(VirtioGpuResourceAttachBacking {
            hdr: VirtioGpuCtrlHdr {
                hdr_type: VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
                ..Default::default()
            },
            resource_id,
            nr_entries: 1,
        });
        let resource = self
            .resources
            .get_mut(&resource_id)
            .ok_or(VirtioError::DeviceError(format!(
                "Invalid resource id: {resource_id}"
            )))?;
        match resource {
            VirtioGpuResource::TwoD {
                width,
                height,
                format,
                addr: ptr,
                size,
            } => {
                // 4 bytes per pixel
                let buf_size = (width * height * 4) as usize;
                let mut buffer = alloc::vec![0; buf_size];
                let buf_ptr = buffer.as_mut_ptr() as usize;
                *ptr = buf_ptr;
                *size = buf_size;
                let mut entry = Box::new(VirtioGpuMemEntry {
                    addr: buf_ptr.addr() as u64,
                    length: buf_size as u32,
                    _padding: 0,
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
                self.controlq
                    .wait_used_timeout(last, Duration::from_secs(1))?;

                if !response.is_ok_nodata() {
                    return Err(VirtioError::ErrorResponse(response.hdr_type));
                }

                return Ok(*response);
            }
            VirtioGpuResource::ThreeD { .. } => {
                return Err(VirtioError::NotImplemented(
                    "VirtioGPU",
                    "3D buffer allocation",
                ));
            }
        }
    }

    fn get_mut_resource(
        &mut self,
        resource_id: u32,
    ) -> Result<&mut VirtioGpuResource, VirtioError> {
        self.resources
            .get_mut(&resource_id)
            .ok_or(VirtioError::DeviceError(format!(
                "Invalid resource id: {resource_id}"
            )))
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
            _padding: 0,
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
            _padding: 0,
        };
        self.controlq
            .push_desc(self.mmio_addr, &flush_req, &mut response);
        Ok(())
    }
}

impl GpuDevice for VirtioGpu {
    /// Creates a resource for the GPU allocating the backing in memory, returns [`Ok`] containing the resourece ID on success
    fn create_resource(&mut self, width: u32, height: u32) -> Result<u32, GpuError> {
        self.register_resource(width, height)
            .map_err(|err| GpuError::ResourceCreateFailed)
    }
    fn destroy_resource(&mut self, resource_id: u32) -> Result<usize, GpuError> {
        self.destroy_resource(resource_id)
            .map_err(|_| GpuError::ResourceDestroyFailed)
    }
    /// query resources by resource id
    fn get_resource(&self, resource_id: u32) -> Option<&VirtioGpuResource> {
        self.resources.get(&resource_id)
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn flush(&mut self) -> Result<(), GpuError> {
        self.flush_resources()
    }

    fn flip(&mut self) -> Result<(), GpuError> {
        todo!()
    }
}
