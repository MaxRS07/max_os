/// A rd-time backed global system time accessor
pub trait GlobalTimer {
    /// Returns the current number of ticks since boot
    fn ticks(&self) -> u64;
    /// The frequency of the timer in ticks/s
    fn freq(&self) -> u64;

    fn now_ms(&self) -> u64 {
        self.ticks() / self.freq() * 1000
    }
    /// Converts microseconds to ticks
    #[inline(always)]
    fn us_to_ticks(&self, us: u64) -> u64 {
        (us.saturating_mul(self.freq())) / 1_000_000
    }

    /// Converts milliseconds to raw timer ticks.
    #[inline(always)]
    fn ms_to_ticks(&self, ms: u64) -> u64 {
        (ms.saturating_mul(self.freq())) / 1_000
    }

    /// whether `ticks` ticks have elapsed since `start` ticks
    #[inline(always)]
    fn elapsed(&self, start: u64, duration: u64) -> bool {
        (self.ticks().wrapping_sub(start)) >= duration
    }
}
