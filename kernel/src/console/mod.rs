use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};
use core::write;

use alloc::fmt;
use log::Level;

use crate::drivers::time;

pub mod writer;

static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn init(debug: bool) {
    DEBUG_ENABLED.store(debug, Ordering::Relaxed);
    log::logger::set_logger(|level, args| {
        if level == Level::Debug && !DEBUG_ENABLED.load(Ordering::Relaxed) {
            return;
        }
        let mut writer = writer::Terminal;
        // pad to align msg for any level
        let padding_str = match level {
            Level::Debug | Level::Error | Level::Trace => " ",
            Level::Info | Level::Warn => "  ",
        };
        let _ = write_hhmmss(&mut writer);
        let _ = write!(writer, " [{:?}]", level);
        let _ = writer.write_str(padding_str);
        let _ = writer.write_fmt(args);
        let _ = writer.write_str("\n");
    });
}
pub fn write_hhmmss(w: &mut dyn Write) -> fmt::Result {
    let time = time::mtime_ms();
    let seconds = time / 1000 % 60;
    let mins = seconds / 60 % 60;
    let hours = mins / 60;
    write!(
        w,
        "{:02}:{:02}:{:02}.{:?>03}",
        hours,
        mins,
        seconds,
        time % 1000
    )
}

pub fn set_debug(debug: bool) {
    DEBUG_ENABLED.store(debug, Ordering::Relaxed);
}
