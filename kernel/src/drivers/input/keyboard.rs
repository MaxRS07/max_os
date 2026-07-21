use core::default;

use alloc::vec::Vec;
use input::{config::VirtioInputEvent, device::KeyboardDevice, state::KeyState};

#[derive(Clone, Debug, Default)]
pub struct GlobalKeyboard {
    state: KeyState,
    event_buffer: Vec<VirtioInputEvent>,
}

impl GlobalKeyboard {
    pub const fn new() -> Self {
        GlobalKeyboard {
            state: KeyState::new(),
            event_buffer: Vec::new(),
        }
    }
    /// updates internal state with keyboard inputs
    pub fn update_keys(&mut self, events: Vec<VirtioInputEvent>) {
        self.event_buffer = events;
        for event in &self.event_buffer {
            self.state.record_key(*event);
        }
    }
}

impl KeyboardDevice for GlobalKeyboard {
    fn keycode_down(&self, keycode: u16) -> bool {
        self.state.get_key_down(keycode)
    }

    fn keycode_pressed(&self, keycode: u16) -> bool {
        self.event_buffer
            .iter()
            .any(|evt| evt.code == keycode && evt.type_ == 1) // 1: pressed code
    }

    fn keycode_released(&self, keycode: u16) -> bool {
        self.event_buffer
            .iter()
            .any(|evt| evt.code == keycode && evt.type_ == 0) // 0: release code
    }
}
