pub mod virtio;

pub const GET_DISPLAY_INFO: u32 = 0x0100;
const RESOURCE_CREATE_2D: u32 = 0x0101;
const RESOURCE_UNREF: u32 = 0x0102;
const SET_SCANOUT: u32 = 0x0103;
const RESOURCE_FLUSH: u32 = 0x0104;
const TRANSFER_TO_HOST_2D: u32 = 0x0105;
const RESOURCE_ATTACH_BACKING: u32 = 0x0106;
const RESOURCE_DETACH_BACKING: u32 = 0x0107;
const RESP_OK_NODATA: u32 = 0x1100;
const RESP_OK_DISPLAY_INFO: u32 = 0x1101;
const RESP_ERR_UNSPEC: u32 = 0x1200;

// limit to 1080p for now
const MAX_RESOLUTION: (usize, usize) = (1920, 1080);

use core::alloc::Layout;

use core_utils::mem::r#box::Box;
use log::{error, info};

use crate::{
    drivers::{
        gpu::virtio::{GpuCtrlHdr, VirtioGpuCtrlHdr, VirtqAvail, VirtqDesc, VirtqUsed},
        mmio::{mmio_read, mmio_write},
    },
    mm::error::MemoryError,
};

#[allow(dead_code)]
static mut FRAMEBUFFER_START: u32 = 0;
#[allow(dead_code)]
static mut FRAMEBUFFER_SIZE: u32 = 0;

pub fn initialize_virtio_gpu(base: usize) {
    // Reset
    mmio_write::<u32>(base, 0x70, 0); // Status = 0

    // ACKNOWLEDGE | DRIVER
    mmio_write::<u32>(base, 0x70, 1);
    mmio_write::<u32>(base, 0x70, 3);

    // Read device features
    mmio_write::<u32>(base, 0x14, 0); // DeviceFeaturesSel = 0
    let features_low = mmio_read::<u32>(base, 0x10);
    mmio_write::<u32>(base, 0x14, 1);
    let features_high = mmio_read::<u32>(base, 0x10);

    let features = ((features_high as u64) << 32) | features_low as u64;
    info!("virtio-gpu features: {:#x}", features);

    // FEATURES_OK
    mmio_write::<u32>(base, 0x70, 11);

    // check FEATURES_OK stuck
    let status = mmio_read::<u32>(base, 0x70);
    if status & 8 == 0 {
        error!("Device rejected features");
        return;
    }

    info!("virtio-gpu ready at {:#x}", base);

    init_virtqueue(base);
}

const QUEUE_SIZE: usize = 16;
const CONTROLQ: u32 = 0;

fn setup_queue(mmio: usize, queue_idx: u32, desc: usize, avail: usize, used: usize) {
    mmio_write::<u32>(mmio, 0x30, queue_idx);
    mmio_write::<u32>(mmio, 0x38, QUEUE_SIZE as u32);

    mmio_write::<u32>(mmio, 0x80, desc as u32);
    mmio_write::<u32>(mmio, 0x84, 0); // high always 0 on 32-bit

    mmio_write::<u32>(mmio, 0x90, avail as u32);
    mmio_write::<u32>(mmio, 0x94, 0);

    mmio_write::<u32>(mmio, 0xA0, used as u32);
    mmio_write::<u32>(mmio, 0xA4, 0);

    mmio_write::<u32>(mmio, 0x44, 1);
}

fn init_virtqueue(mmio_addr: usize) {
    let desc = Box::new([VirtqDesc::default(); QUEUE_SIZE]);
    let avail = Box::new(VirtqAvail::default());
    let used = Box::new(VirtqUsed::default());

    let request = GpuCtrlHdr {
        hdr_type: GET_DISPLAY_INFO,
        ..Default::default()
    };
    let mut response = GpuCtrlHdr::default();

    unsafe {
        let desc_mut = desc.get_mut();
        desc_mut[0].addr = &request as *const _ as u64;
        desc_mut[0].len = size_of::<GpuCtrlHdr>() as u32; // was size_of::<VirtqDesc>() — wrong
        desc_mut[0].flags = 0x1;
        desc_mut[0].next = 1;

        desc_mut[1].addr = &mut response as *mut _ as u64;
        desc_mut[1].len = size_of::<GpuCtrlHdr>() as u32; // same fix
        desc_mut[1].flags = 0x2;
        desc_mut[1].next = 0;

        // Put desc[0] into the available ring
        let avail_mut = avail.get_mut();
        let idx = avail_mut.idx as usize % QUEUE_SIZE;
        avail_mut.ring[idx] = 0;
        avail_mut.idx = avail_mut.idx.wrapping_add(1);

        // Notify device: QueueNotify, queue index 0
        mmio_write::<u32>(mmio_addr, 0x50, 0);

        // Spin until device consumes the descriptor
        let used_mut = used.get_mut();
        let last_used_idx = used_mut.idx;
        loop {
            if used_mut.idx != last_used_idx {
                break;
            }
            core::hint::spin_loop();
        }

        setup_queue(mmio_addr, 0, desc.addr(), avail.addr(), used.addr());

        allocate_framebuffer();
    }
}

fn allocate_framebuffer() -> Result<usize, MemoryError> {
    let (width, height) = MAX_RESOLUTION;
    // rgba for each pixel
    let fb_size = width * height * 4;
    if let Ok(fb_layout) = Layout::from_size_align(fb_size, 8) {
        unsafe {
            let fb_start = alloc::alloc::alloc(fb_layout);
            Ok(fb_start.addr())
        }
    } else {
        Err(MemoryError::InvalidAddress(
            "Failed to create framebuffer layout",
        ))
    }
}
