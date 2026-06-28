use crate::drivers::uart::write_byte;

use core::fmt::{self, Write};

pub fn print(s: &str) {
    for char in s.bytes() {
        write_byte(char);
    }
}
pub fn println(s: &str) {
    for char in s.bytes() {
        write_byte(char);
    }
    write_byte(10u8);
}
pub struct Terminal;

impl Write for Terminal {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            write_byte(byte);
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let mut writer = Terminal;
    let _ = writer.write_fmt(args);
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::console::terminal::_print("");
    };

    ($fmt:expr $(, $arg:tt)* $(,)?) => {
        {
            $crate::console::terminal::_print(format_args!(concat!($fmt, "\n") $(, $arg)*));
        }
    };
}
