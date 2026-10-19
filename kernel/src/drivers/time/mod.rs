use sync::oncelock::OnceLock;
use systime::global_time::GlobalTimer;

use crate::drivers::time::sysclock::SystemClock;

pub mod sysclock;
pub mod timer;

#[unsafe(no_mangle)]
pub static __GLOBAL_TIMER: SystemClock = SystemClock;
