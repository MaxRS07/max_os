use core::{cell::UnsafeCell, fmt::Debug, ops::BitOrAssign};

/// common api-facing input device for mouse and keyboard probing
pub trait KeyboardDevice: Debug {
    fn keycode_down(&self, keycode: u16) -> bool;
    fn keycode_pressed(&self, keycode: u16) -> bool;
    fn keycode_released(&self, keycode: u16) -> bool;
}
pub trait MouseDevice: Debug {
    fn mouse_down(&self, mouse_btn: u16) -> bool;
    fn mouse_pressed(&self, mouse_btn: u16) -> bool;
    fn mouse_released(&self, mouse_btn: u16) -> bool;
    fn mouse_position(&self) -> (i32, i32);
    fn mouse_delta(&self) -> (i32, i32);
}

const NONE: u8 = 0;
const MOUSE: u8 = 1;
const KEYBOARD: u8 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DeviceType(u8);

impl DeviceType {
    pub const NONE: DeviceType = DeviceType(NONE);
    pub const MOUSE: DeviceType = DeviceType(MOUSE);
    pub const KEYBOARD: DeviceType = DeviceType(KEYBOARD);

    pub fn is_keyboard(&self) -> bool {
        self.0 & KEYBOARD != 0
    }
    pub fn is_mouse(&self) -> bool {
        self.0 & MOUSE != 0
    }
}

impl BitOrAssign for DeviceType {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

impl core::fmt::Debug for DeviceType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let string = match self.0 {
            MOUSE => "MOUSE",
            KEYBOARD => "KEYBOARD",
            3 => "MULTI-INPUT",
            _ => "NONE",
        };
        f.write_str(string)
    }
}
