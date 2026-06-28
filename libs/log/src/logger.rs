use crate::Level;
use core::fmt::Arguments;

static mut LOGGER: Option<fn(Level, Arguments)> = None;

pub fn set_logger(f: fn(Level, Arguments)) {
    unsafe {
        LOGGER = Some(f);
    }
}

pub fn log(level: Level, args: Arguments) {
    unsafe {
        if let Some(f) = LOGGER {
            f(level, args);
        }
    }
}
