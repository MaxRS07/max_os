use crate::config::VirtioInputEvent;

/// Holds a keyboard input state
#[derive(Clone, Copy, Debug)]
pub struct KeyState {
    key_down: [bool; 256],
}

impl KeyState {
    /// Blank state (all keys are up)
    pub const fn new() -> Self {
        KeyState {
            key_down: [false; 256],
        }
    }
    pub fn record_key(&mut self, event: VirtioInputEvent) {}
    pub fn get_key_down(&self, key: u16) -> bool {
        self.key_down[key as usize]
    }
}

impl Default for KeyState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MouseState {
    x: u32,
    y: u32,
    /// TODO: Find out how many keycodes there actually are on a mouse
    mouse_down: [bool; 81],
}
impl MouseState {
    pub fn new() -> Self {
        MouseState {
            x: 0,
            y: 0,
            mouse_down: [false; 81],
        }
    }
}

impl Default for MouseState {
    fn default() -> Self {
        Self::new()
    }
}
