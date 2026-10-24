use core::{
    alloc::Layout,
    convert::identity,
    net::AddrParseError,
    ptr::NonNull,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format, slice,
    string::String,
    vec::{self, Vec},
};
use fdt::tree::FDT;
use graphics::{
    device::driver::DisplayDriver, error::DisplayError, primitives::color::ColorFormat,
};
use log::{debug, error, warn};
use mem::error::MemoryError;
use virtio::{
    ALL_FEATURES,
    error::VirtioError,
    types::{
        VirtioGpuCtrlHdr, VirtioGpuDisplayInfo, VirtioGpuMemEntry, VirtioGpuRect,
        VirtioGpuResourceAttachBacking, VirtioGpuResourceCreate2D, VirtioGpuResourceCreate3D,
        VirtioGpuResourceFlush, VirtioGpuResourceUnref, VirtioGpuSetScanout,
        VirtioGpuTransferToHost2d,
        device::VirtioDevice,
        queue::{DescBuf, VirtQueue},
    },
};

use crate::drivers::gpu::virtio::cmd::{
    VIRTIO_GPU_CMD_GET_DISPLAY_INFO, VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
    VIRTIO_GPU_CMD_RESOURCE_CREATE_2D, VIRTIO_GPU_CMD_RESOURCE_FLUSH,
    VIRTIO_GPU_CMD_RESOURCE_UNREF, VIRTIO_GPU_CMD_SET_SCANOUT, VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D,
    VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM,
};

pub const MAX_SURFACES: usize = 16;

// adapted from specification here: https://docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html#x1-3960007
pub mod cmd;

struct VirtioGpuResource {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub backing: Backing,
}

struct Backing {
    pub ptr: NonNull<u8>,
    pub len: usize,
    pub phys: u64, // what you hand to the device
}

impl Backing {
    fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}
impl Drop for Backing {
    fn drop(&mut self) {}
}
/// VirtIO GPU Device
pub struct VirtioGpu {
    device: VirtioDevice,
    width: u32,
    height: u32,
    features: u64,
    controlq: VirtQueue,
    cursorq: VirtQueue,
    resources: [Option<VirtioGpuResource>; MAX_SURFACES],
}

impl VirtioGpu {
    pub fn new(fdt: &FDT, mmio_idx: usize) -> Result<Self, VirtioError> {
        let mut gpu_device = VirtioDevice::probe(fdt, mmio_idx, 16)?;

        let features = gpu_device.select_features(u64::MAX)?;

        let controlq = gpu_device.setup_queue(0)?;
        let cursorq = gpu_device.setup_queue(1)?;

        gpu_device.finish();

        let mut virtio_gpu = VirtioGpu {
            device: gpu_device,
            controlq,
            cursorq,
            width: 0,
            height: 0,
            features,
            resources: [const { Option::None }; MAX_SURFACES],
        };

        let info = virtio_gpu.get_display_info()?;
        let rect = info.pmodes[0].r;

        debug!("initialized framebuffer");
        virtio_gpu.scanout(1, rect);
        if let Err(error) = virtio_gpu.flush_resources() {
            error!("Failed to flush VirtIO GPU: {}", error)
        }
        debug!("flushed resources");

        Ok(virtio_gpu)
    }

    fn get_display_info(&mut self) -> Result<VirtioGpuDisplayInfo, VirtioError> {
        let request = VirtioGpuCtrlHdr {
            hdr_type: VIRTIO_GPU_CMD_GET_DISPLAY_INFO,
            ..Default::default()
        };
        let mut response = VirtioGpuDisplayInfo::default();

        self.controlq.push_desc_timeout(
            self.device.mmio,
            &request,
            &mut response,
            Duration::from_secs(1),
        );
        if !response.hdr.is_ok_nodata() {
            return Err(VirtioError::ErrorResponse(response.hdr.hdr_type));
        }
        Ok(response)
    }

    fn register_resource(
        &mut self,
        width: u32,
        height: u32,
        format: ColorFormat,
        backing: Backing,
    ) -> Result<u32, VirtioError> {
        let resource_id = self.create_resource2d(width, height, format, backing)?;
        self.attach_backing(resource_id)?;
        Ok(resource_id)
    }

    fn create_resource2d(
        &mut self,
        width: u32,
        height: u32,
        format: ColorFormat,
        backing: Backing,
    ) -> Result<u32, VirtioError> {
        let resource_id = self.next_id()?;
        let mut request = VirtioGpuResourceCreate2D::default();
        request.hdr.hdr_type = VIRTIO_GPU_CMD_RESOURCE_CREATE_2D;
        request.resource_id = resource_id as u32;
        request.format = VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM;
        request.width = width;
        request.height = height;

        let mut response = VirtioGpuCtrlHdr::default();

        self.controlq.push_desc_timeout(
            self.device.mmio,
            &request,
            &mut response,
            Duration::from_secs(1),
        )?;

        if !response.is_ok_nodata() {
            return Err(VirtioError::ErrorResponse(response.hdr_type));
        }

        self.resources[resource_id - 1] = Some(VirtioGpuResource {
            width,
            height,
            format: VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM,
            backing,
        });

        Ok(resource_id as u32)
    }
    /// returns the next free resource id
    fn next_id(&self) -> Result<usize, VirtioError> {
        let next =
            self.resources
                .iter()
                .position(|r| r.is_none())
                .ok_or(VirtioError::DeviceError(format!(
                    "Failed to create surface, free slots exhausted"
                )))?;
        Ok(next + 1)
    }
    // pub fn create_cursor_resource(&mut self) -> Result<u32, VirtioError> {
    //     let cursor_id = self.create_resource2d(64, 64, ColorFormat::Rgba, )?;
    //     self.attach_backing(cursor_id)?;
    //     Ok(cursor_id)
    // }
    /// Removes a resource from this gpu, deallocating the attached backing. Returns [`Ok`] on success conraining the size of the freed backing in bytes.
    fn destroy_resource(&mut self, resource_id: u32) -> Result<usize, VirtioError> {
        let mut request = VirtioGpuResourceUnref::default();
        request.hdr.hdr_type = VIRTIO_GPU_CMD_RESOURCE_UNREF;
        request.resource_id = resource_id;

        let mut response = VirtioGpuCtrlHdr::default();

        self.controlq.push_desc_timeout(
            self.device.mmio,
            &request,
            &mut response,
            Duration::from_secs(1),
        )?;
        self.resources[resource_id as usize - 1] = None;
        Ok(0)
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
        // let resource = self.resources[resource_id as usize - 1]
        //     .ok_or(VirtioError::DeviceError(String::new()))?;

        let mut entry = Box::new(VirtioGpuMemEntry {
            addr: 0 as u64,
            length: 0 as u32,
            _padding: 0,
        });
        let mut response = Box::new(VirtioGpuCtrlHdr::default());

        let req_addr = &mut *request as *mut VirtioGpuResourceAttachBacking as usize;
        let entry_addr = &mut *entry as *mut VirtioGpuMemEntry as usize;
        let address = &mut *response as *mut VirtioGpuCtrlHdr as usize;

        let last = self.controlq.push_desc_raw(
            self.device.mmio,
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

    fn get_mut_resource(
        &mut self,
        resource_id: u32,
    ) -> Result<&mut VirtioGpuResource, VirtioError> {
        self.resources[resource_id as usize]
            .as_mut()
            .ok_or(VirtioError::DeviceError(format!(
                "Invalid resource id: {resource_id}"
            )))
    }

    fn get_resource(&self, resource_id: u32) -> Result<&VirtioGpuResource, VirtioError> {
        self.resources[resource_id as usize]
            .as_ref()
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
            self.device.mmio,
            &request,
            &mut response,
            Duration::from_secs(1),
        )
    }

    fn flush_resources(&mut self) -> Result<(), DisplayError> {
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
            .push_desc(self.device.mmio, &transfer_req, &mut response);

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
            .push_desc(self.device.mmio, &flush_req, &mut response);
        Ok(())
    }
}

impl DisplayDriver for VirtioGpu {
    fn info(&self) -> graphics::device::display::DisplayInfo {
        todo!()
    }

    fn requirements(&self) -> graphics::device::backing::BackingMode {
        todo!()
    }

    fn create_resource(
        &mut self,
        desc: &graphics::device::resource::ResourceDesc,
        backing: &graphics::device::backing::Backing,
    ) -> Result<graphics::device::resource::ResourceHandle, DisplayError> {
        todo!()
    }

    fn destroy_resource(
        &mut self,
        h: graphics::device::resource::ResourceHandle,
    ) -> Result<(), DisplayError> {
        todo!()
    }

    fn scanout(
        &mut self,
        h: graphics::device::resource::ResourceHandle,
    ) -> Result<(), DisplayError> {
        todo!()
    }

    fn flush(
        &mut self,
        h: graphics::device::resource::ResourceHandle,
        rect: Option<graphics::primitives::rect::ScreenRect>,
    ) -> Result<(), DisplayError> {
        todo!()
    }

    fn wait(&mut self) -> Result<(), DisplayError> {
        todo!()
    }

    fn set_cursor(&mut self, x: u32, y: u32) -> Result<(), DisplayError> {
        todo!()
    }
}
