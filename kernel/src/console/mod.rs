use core::fmt::Write;

use crate::println;

pub mod shell;
pub mod terminal;

pub fn init() {
    log::logger::set_logger(|level, args| {
        let mut writer = terminal::Terminal;

        // 1. Write the static time and log level metadata prefix directly
        let _ = write!(writer, "00:00:00 [{:?}] ", level);

        // 2. Directly unpack and stream the macro arguments payload to the UART pipeline
        let _ = writer.write_fmt(args);

        // 3. Append the final trailing newline character byte safely
        let _ = writer.write_str("\n");
    });
}
