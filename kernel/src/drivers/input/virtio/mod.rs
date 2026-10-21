use core::{mem, ptr::addr_of};

use alloc::boxed::Box;
use alloc::vec::Vec;
use fdt::tree::FDT;
use input::{
    device::DeviceType,
    event_codes::{ABS_X, ABS_Y, KEY_A, REL_X, REL_Y},
};
use log::{debug, error, info, warn};
use mmio::{mmio_read, mmio_write};
use virtio::{
    ALL_FEATURES, QUEUE_SIZE,
    error::VirtioError,
    types::{
        device::VirtioDevice,
        queue::{DescBuf, VirtQueue},
    },
};

use input::config::{
    CONFIG_BASE, EV_ABS, EV_KEY, EV_REL, VIRTIO_INPUT_CFG_EV_BITS, VIRTIO_INPUT_QUEUE_EVENT,
    VIRTIO_INPUT_QUEUE_STATUS, VirtioInputConfig, VirtioInputEvent,
};

use sync::{mutex::Mutex, oncelock::OnceLock, shared_cell::SharedCell};

use crate::{
    arch::riscv::interrupt::{
        self,
        notifier::INTERRUPT_NOTIFIER,
        route::{InterruptCode, Trap},
        set_interrupt_priority,
    },
    drivers::input::KEYBOARD,
};

static INPUT_DEVICES: OnceLock<Mutex<Vec<VirtioInput>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct VirtioInput {
    device: VirtioDevice,
    device_type: DeviceType,
    status_queue: VirtQueue,
    event_queue: VirtQueue,
    /// actual events in memory
    event_buffer: Box<[VirtioInputEvent; QUEUE_SIZE]>,
}

impl VirtioInput {
    pub fn new(fdt: &FDT, mmio_idx: usize) -> Result<DeviceType, VirtioError> {
        let mutex_devices = INPUT_DEVICES.get_or_set(Mutex::new(Vec::new()));
        let mut device = VirtioDevice::probe(fdt, mmio_idx, 2)?;

        let _ = device.select_features(ALL_FEATURES)?;

        let empty_buffer = [VirtioInputEvent::default(); QUEUE_SIZE];

        let event_queue = device.setup_queue(VIRTIO_INPUT_QUEUE_EVENT)?;
        let status_queue = device.setup_queue(VIRTIO_INPUT_QUEUE_STATUS)?;

        device.finish();

        let mut input = VirtioInput {
            device,
            device_type: DeviceType::NONE,
            status_queue,
            event_queue,
            event_buffer: Box::new(empty_buffer),
        };
        input.set_device_type();
        input.populate_event_queue();

        let device_type = input.device_type;

        let device_idx = unsafe {
            let mut devices = mutex_devices.lock();
            if devices.capacity() == 0 {
                devices.reserve(8);
            }
            devices.push(input);
            devices.len() - 1
        };

        info!("configuring notifier");

        let irq_id = mmio::get_interrupt(fdt, mmio_idx).unwrap_or(mmio_idx as u32);
        set_interrupt_priority(fdt, irq_id, 1);

        info!("Attempting lock on interrupt notifier...");

        INTERRUPT_NOTIFIER.wait().lock().subscribe(irq_id, move || {
            if let Some(cell) = INPUT_DEVICES.wait().lock().get_mut(device_idx) {
                cell.handle_interrupt();
            }
        });

        Ok(device_type)
    }

    pub fn handle_interrupt(&mut self) {
        let status = mmio_read::<u32>(self.device.mmio, 0x60);

        if status == 0 {
            return;
        }

        if (status & 0x1) != 0 {
            let events = self.poll_events();
            if self.device_type == DeviceType::KEYBOARD {
                unsafe { KEYBOARD.get_mut().update_keys(events) }
            }
        }

        if (status & 0x2) != 0 {
            // configuration changed
            // self.handle_config_change();
        }

        // write back to acknowledge interrupt recived
        mmio_write(self.device.mmio, 0x64, status);
    }
    pub fn device_type(&self) -> DeviceType {
        self.device_type
    }
    fn get_input_config(&self, select: u8, subsel: u8) -> VirtioInputConfig {
        // reset first for change detection
        mmio_write::<u8>(self.device.mmio, CONFIG_BASE, 0);
        mmio_write::<u8>(self.device.mmio, CONFIG_BASE + 1, 0);
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        // write correct select values
        mmio_write::<u8>(self.device.mmio, CONFIG_BASE, select);
        mmio_write::<u8>(self.device.mmio, CONFIG_BASE + 1, subsel);
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        let cfg_ptr = (self.device.mmio + CONFIG_BASE as usize) as *const VirtioInputConfig;
        unsafe { cfg_ptr.read_unaligned() }
    }
    /// sets the device for supported event types
    fn set_device_type(&mut self) {
        let key_cfg = self.get_input_config(VIRTIO_INPUT_CFG_EV_BITS, EV_KEY); // buttons/keys
        let rel_cfg = self.get_input_config(VIRTIO_INPUT_CFG_EV_BITS, EV_REL); // relative movement
        let abs_cfg = self.get_input_config(VIRTIO_INPUT_CFG_EV_BITS, EV_ABS); // absolute movement

        let key_bitmap = unsafe { key_cfg.u.bitmap };
        let rel_bitmap = unsafe { rel_cfg.u.bitmap };
        let abs_bitmap = unsafe { abs_cfg.u.bitmap };

        let mut device_type = DeviceType::NONE;

        // keyboard: has letter keys
        if Self::check_bitmap(&key_bitmap, KEY_A) {
            device_type |= DeviceType::KEYBOARD;
        }

        // mouse: has relative or absolute axes
        let has_rel =
            Self::check_bitmap(&rel_bitmap, REL_X) || Self::check_bitmap(&rel_bitmap, REL_Y);
        let has_abs =
            Self::check_bitmap(&abs_bitmap, ABS_X) && Self::check_bitmap(&abs_bitmap, ABS_Y);
        if has_rel || has_abs {
            device_type |= DeviceType::MOUSE;
        }

        self.device_type = device_type;
    }
    /// populate event queue with empty input structs
    fn populate_event_queue(&mut self) {
        for i in 0..self.event_buffer.len() {
            let evt_ptr = addr_of!(self.event_buffer[i]);
            let buf = [DescBuf {
                addr: evt_ptr.addr() as u64,
                len: mem::size_of::<VirtioInputEvent>() as u32,
                flags: 2,
            }];
            // Fire-and-forget: these are standing receive buffers that only
            // complete once a real input event arrives, which may never
            // happen, so we must not wait on the used ring here.
            self.event_queue
                .push_desc_raw(self.device.mmio, &buf, i as u16);
        }
    }
    /// returns a Vec of all input events and resets the buffer
    pub fn poll_events(&mut self) -> Vec<VirtioInputEvent> {
        let mut events = Vec::new();

        while let Some((desc_idx, _len)) = self.event_queue.pop_used() {
            let event = self.event_buffer[desc_idx as usize];
            events.push(event);

            self.event_buffer[desc_idx as usize] = VirtioInputEvent::default();
            self.event_queue.avail_push(desc_idx);
        }
        if !events.is_empty() {
            self.event_queue.notify(self.device.mmio);
        }
        events
    }
    fn check_bitmap(bitmap: &[u8; 128], code: u16) -> bool {
        let byte = code / 8;
        let bit = code % 8;
        // fast bitwise check
        bitmap[byte as usize] & (1 << bit) != 0
    }
}
