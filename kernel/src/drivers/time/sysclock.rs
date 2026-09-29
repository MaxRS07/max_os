use time::global_time::GlobalTimer;

use crate::drivers::time::timer;

pub struct SystemClock;

impl GlobalTimer for SystemClock {
    fn ticks(&self) -> u64 {
        timer::get_upticks()
    }
    fn freq(&self) -> u64 {
        // this is constant for QEMU
        10_000_000
    }
}
