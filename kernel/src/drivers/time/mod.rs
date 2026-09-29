use sync::oncelock::OnceLock;
use time::global_time::GlobalTimer;

use crate::drivers::time::sysclock::SystemClock;

pub mod sysclock;
pub mod timer;

pub static GLOBAL_TIME: OnceLock<SystemClock> = OnceLock::new();

pub fn start_system_clock() {
    GLOBAL_TIME.set(SystemClock {});
}
