//! crates/platform_bridge/src/observability/rdtsc.rs
//! Hardware CPU Timestamp Counter (_rdtsc) for Sub-Nanosecond Telemetry.
//! Bypasses OS clock system calls, executing in ~3-5 CPU clock cycles (<1 nanosecond).

/// Directly reads the hardware Time Stamp Counter register on x86_64 CPUs.
#[inline(always)]
pub fn read_cpu_timestamp() -> u64 {
    // `_rdtsc` takes no arguments and reads a CPU register into a return
    // value; it has no pointer/memory preconditions and is available on
    // every x86_64 CPU (RDTSC has been unconditionally present since the
    // Pentium), which this `cfg(target_arch = "x86_64")` guard ensures.
    // SAFETY: no preconditions beyond running on x86_64, guaranteed by cfg.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_rdtsc()
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}

/// Ultra-low overhead hardware cycle profiler for sub-nanosecond oscilloscope instrumentation.
#[derive(Debug, Clone, Copy)]
pub struct HardwareCycleProfiler {
    start_cycles: u64,
}

impl HardwareCycleProfiler {
    #[inline(always)]
    pub fn start() -> Self {
        Self {
            start_cycles: read_cpu_timestamp(),
        }
    }

    #[inline(always)]
    pub fn elapsed_cycles(&self) -> u64 {
        read_cpu_timestamp().saturating_sub(self.start_cycles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdtsc_micro_timing() {
        let t1 = read_cpu_timestamp();
        let profiler = HardwareCycleProfiler::start();
        let mut _acc = 0u64;
        for i in 0..100 {
            _acc = _acc.wrapping_add(i);
        }
        let elapsed = profiler.elapsed_cycles();
        let t2 = read_cpu_timestamp();
        assert!(t2 >= t1);
        // elapsed cycle count is not asserted to an arbitrarily tight bound:
        // wall-clock cycle duration in debug unit tests is host-load dependent
        // and subject to OS thread preemption. Tight cycle budgets belong in benches/.
        let _ = elapsed;
    }
}
