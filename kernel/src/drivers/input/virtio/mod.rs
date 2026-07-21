use core::mem;

use alloc::boxed::Box;
use alloc::vec::Vec;
use input::{
    device::DeviceType,
    event_codes::{ABS_X, ABS_Y, KEY_A, REL_X, REL_Y},
};
use log::{error, info, warn};
use mmio::{mmio_read, mmio_write};
use sdt::fdt::FDT;
use virtio::{
    QUEUE_SIZE, VIRTIO_MMIO_MAGIC_VALUE, init::init_state, queue::init_virtqueue,
    types::queue::VirtQueue,
};

use input::config::{
    CONFIG_BASE, EV_ABS, EV_KEY, EV_REL, VIRTIO_INPUT_CFG_EV_BITS, VIRTIO_INPUT_QUEUE_EVENT,
    VIRTIO_INPUT_QUEUE_STATUS, VirtioInputConfig, VirtioInputEvent,
};

use sync::{once::Once, shared_cell::SharedCell};

use crate::{
    arch::riscv::interrupt::{
        self,
        notifier::INTERRUPT_NOTIFIER,
        route::{InterruptCode, Trap},
        set_interrupt_priority,
    },
    drivers::input::KEYBOARD,
};

/// Global home for the input device so `'static` interrupt callbacks can reach
/// it. `SharedCell` provides the interior mutability `handle_interrupt` needs.
static INPUT_DEVICES: SharedCell<Vec<VirtioInput>> = SharedCell::new(Vec::new());

#[derive(Debug, Clone)]
pub struct VirtioInput {
    device_type: DeviceType,
    mmio_addr: usize,
    status_queue: VirtQueue,
    event_queue: VirtQueue,
    /// actual events in memory
    event_buffer: Box<[VirtioInputEvent; QUEUE_SIZE]>,
    last_idx: u16,
}

impl VirtioInput {
    pub fn from_mmio(fdt: &FDT, mmio_idx: usize) -> Option<DeviceType> {
        let mmio_addr = fdt.virtio_mmio[mmio_idx].base_address;
        let version = mmio_read::<u32>(mmio_addr, 0x04);
        let magic = mmio_read::<u32>(mmio_addr, 0x00);
        if magic != VIRTIO_MMIO_MAGIC_VALUE || version != 2 {
            return None;
        }
        info!("VirtIO input device detected");

        let empty_buffer = [VirtioInputEvent::default(); QUEUE_SIZE];
        let mut input = VirtioInput {
            mmio_addr,
            device_type: DeviceType::NONE,
            status_queue: VirtQueue::default(),
            event_queue: VirtQueue::default(),
            event_buffer: Box::new(empty_buffer),
            last_idx: 0,
        };
        if let Err(error) = init_state(mmio_addr) {
            error!("{}", error);
            return None;
        }
        input.set_device_type();

        // Setup event queue
        if let Err(error) =
            init_virtqueue(&mut input.event_queue, mmio_addr, VIRTIO_INPUT_QUEUE_EVENT)
        {
            error!("Failed to initialize event queue: {}", error)
        }
        input.populate_event_queue();

        // Setup status queue
        if let Err(error) = init_virtqueue(
            &mut input.status_queue,
            mmio_addr,
            VIRTIO_INPUT_QUEUE_STATUS,
        ) {
            error!("Failed to initialize status queue: {}", error)
        }
        // signal DRIVER_OK
        mmio_write::<u32>(mmio_addr, 0x70, 15);

        let device_type = input.device_type;

        let device_idx = unsafe {
            let devices = INPUT_DEVICES.get_mut();
            // Reserve up front so later pushes never reallocate (and move) a
            // device while an already-armed device's interrupt is reading the
            // Vec. Only allocates once, when the Vec is still empty.
            if devices.capacity() == 0 {
                devices.reserve(8);
            }
            devices.push(input);
            devices.len() - 1
        };

        let irq_id = mmio::get_interrupt(fdt, mmio_idx).unwrap_or(mmio_idx as u32);
        set_interrupt_priority(fdt, irq_id, 1);

        if let Some(notifer) = unsafe { INTERRUPT_NOTIFIER.get_mut() } {
            // Route this IRQ to the device we just registered, not a shared
            // global. `device_idx` is captured by value so each subscription
            // services its own device.
            notifer.subscribe(irq_id, move || {
                if let Some(cell) = unsafe { INPUT_DEVICES.get_mut().get_mut(device_idx) } {
                    cell.handle_interrupt();
                }
            });
        }

        Some(device_type)
    }

    pub fn handle_interrupt(&mut self) {
        let status = mmio_read::<u32>(self.mmio_addr, 0x60);

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
        mmio_write(self.mmio_addr, 0x64, status);
    }
    pub fn device_type(&self) -> DeviceType {
        self.device_type
    }
    fn get_input_config(&self, select: u8, subsel: u8) -> VirtioInputConfig {
        // reset first for change detection
        mmio_write::<u8>(self.mmio_addr, CONFIG_BASE, 0);
        mmio_write::<u8>(self.mmio_addr, CONFIG_BASE + 1, 0);
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        // write correct select values
        mmio_write::<u8>(self.mmio_addr, CONFIG_BASE, select);
        mmio_write::<u8>(self.mmio_addr, CONFIG_BASE + 1, subsel);
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        let cfg_ptr = (self.mmio_addr + CONFIG_BASE as usize) as *const VirtioInputConfig;
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
        for i in 0..QUEUE_SIZE {
            let evt_ptr = &self.event_buffer[i] as *const VirtioInputEvent;

            let desc_queue = self.event_queue.desc.as_mut();
            desc_queue[i].addr = evt_ptr.addr() as u64;
            desc_queue[i].len = mem::size_of::<VirtioInputEvent>() as u32;
            // allow virtio to write to this block
            desc_queue[i].flags = 2;

            let avail_queue = self.event_queue.avail.as_mut();
            avail_queue.ring[i] = i as u16;
        }
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

        let avail_queue = self.event_queue.avail.as_mut();
        avail_queue.idx = avail_queue.idx.wrapping_add(QUEUE_SIZE as u16);

        // Notify device
        mmio_write(self.mmio_addr, 0x50, 0);
    }
    /// returns a Vec of all input events and resets the buffer
    pub fn poll_events(&mut self) -> Vec<VirtioInputEvent> {
        let mut events = Vec::new();
        let used = self.event_queue.used.as_mut();

        while self.last_idx != used.idx {
            let used_slot = (self.last_idx as usize) % QUEUE_SIZE;
            let desc_idx = used.ring.get(used_slot).unwrap().id;

            let event = self.event_buffer[desc_idx as usize];
            events.push(event);

            self.event_buffer[desc_idx as usize] = VirtioInputEvent::default();

            let avail = self.event_queue.avail.as_mut();
            let avail_slot = avail.idx as usize % QUEUE_SIZE;
            avail.ring[avail_slot] = desc_idx as u16;

            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

            avail.idx = avail.idx.wrapping_add(1);
            self.last_idx = self.last_idx.wrapping_add(1);
        }
        if !events.is_empty() {
            mmio_write(self.mmio_addr, 0x50, 0);
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
