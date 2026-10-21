use fdt::tree::FDT;
use input::device::KeyboardDevice;
use log::{error, info};
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
    match virtio::VirtioInput::new(fdt, mmio_idx) {
        Ok(device_type) => info!("Initialized VirtIO {:?} device", device_type),
        Err(error) => error!("Failed to initialize input driver: {error}"),
    }
}
