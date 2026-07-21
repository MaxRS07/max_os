use input::device::KeyboardDevice;
use log::info;
use sdt::fdt::FDT;
use sync::shared_cell::SharedCell;

use crate::drivers::input::keyboard::GlobalKeyboard;

mod keyboard;
mod mouse;
mod virtio;

/// global keyboard
pub static KEYBOARD: SharedCell<GlobalKeyboard> = SharedCell::new(GlobalKeyboard::new());

/// global mouse
// pub static MOUSE: SharedCell<dyn KeyboardDevice>;

pub fn init(fdt: &FDT, mmio_idx: usize) {
    if let Some(device_type) = virtio::VirtioInput::from_mmio(fdt, mmio_idx) {
        info!("Initialized VirtIO {:?} device", device_type);
    }
}
