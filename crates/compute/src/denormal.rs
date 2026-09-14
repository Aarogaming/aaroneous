//! crates/compute/src/denormal.rs
//! Hardware DAZ/FTZ Denormal (Subnormal) Flushing for Recurrent SSM Computations.
//!
//! # Architecture & Rationale
//! In recurrent State-Space Models (Mamba / S4 / SSM), hidden state vectors $h_t = \bar{A} h_{t-1} + \bar{B} x_t$
//! undergo continuous exponential decay via $\bar{A} = \exp(\Delta A)$.
//! When floating-point activations fall into the subnormal / denormal regime ($|x| < 1.175 \times 10^{-38}$ for f32),
//! x86/x86_64 hardware generates internal microcode traps. These traps cause 100x latency spikes
//! (from ~1 ns to 100+ ns per instruction), inducing severe jitter on the 120 Hz supervisory loop.
//!
//! Setting the hardware DAZ (Denormals-Are-Zero) and FTZ (Flush-To-Zero) bits in the MXCSR register
//! forces hardware ALUs to treat subnormals as zero at register level with 0 ns penalty.
//!
//! This module provides an RAII [`DenormalGuard`] that activates DAZ and FTZ upon instantiation
//! and restores the CPU's prior floating-point control state upon [`Drop`].

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
use core::marker::PhantomData;

/// Mask for the DAZ (Denormals-Are-Zero) bit in the MXCSR register (Bit 6 = 0x0040).
pub const MXCSR_DAZ_MASK: u32 = 0x0040;

/// Mask for the FTZ (Flush-To-Zero) bit in the MXCSR register (Bit 15 = 0x8000).
pub const MXCSR_FTZ_MASK: u32 = 0x8000;

/// Combined mask for both DAZ and FTZ bits (0x8040).
pub const MXCSR_DENORMAL_MASK: u32 = MXCSR_DAZ_MASK | MXCSR_FTZ_MASK;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
unsafe fn read_mxcsr() -> u32 {
    let mut mxcsr: u32 = 0;
    // SAFETY: Reading the 32-bit MXCSR floating point control register into stack memory.
    unsafe {
        core::arch::asm!("stmxcsr [{}]", in(reg) &mut mxcsr, options(nostack, preserves_flags));
    }
    mxcsr
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
unsafe fn write_mxcsr(val: u32) {
    // SAFETY: Loading the 32-bit MXCSR floating point control register from stack memory.
    unsafe {
        core::arch::asm!("ldmxcsr [{}]", in(reg) &val, options(nostack, preserves_flags));
    }
}

/// RAII guard for hardware denormal flushing.
///
/// On x86 and x86_64, captures the caller's previous MXCSR register configuration on creation,
/// asserts both FTZ and DAZ bits, and restores the original register configuration upon drop.
/// On non-x86 targets, acts as a zero-overhead no-op marker.
#[derive(Debug)]
pub struct DenormalGuard {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    prev_mxcsr: u32,
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    _marker: PhantomData<()>,
}

impl DenormalGuard {
    /// Creates a new `DenormalGuard`, activating hardware DAZ and FTZ flushing.
    ///
    /// The previous control register state is saved and will be restored when this guard is dropped.
    #[inline]
    pub fn new() -> Self {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // SAFETY: Reading MXCSR via hardware instruction is a safe hardware inspection.
            let prev_mxcsr = unsafe { read_mxcsr() };
            let new_mxcsr = prev_mxcsr | MXCSR_DENORMAL_MASK;
            // SAFETY: Updating MXCSR to enable DAZ and FTZ is safe and only affects floating-point rounding/denormals for this hardware thread.
            unsafe { write_mxcsr(new_mxcsr) };
            Self { prev_mxcsr }
        }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        {
            Self {
                _marker: PhantomData,
            }
        }
    }

    /// Checks if hardware denormal flushing (both DAZ and FTZ) is currently active.
    #[inline]
    pub fn is_active() -> bool {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // SAFETY: Querying MXCSR register flags is side-effect-free on x86/x86_64.
            let mxcsr = unsafe { read_mxcsr() };
            (mxcsr & MXCSR_DENORMAL_MASK) == MXCSR_DENORMAL_MASK
        }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        {
            false
        }
    }

    /// Creates a new `DenormalGuard`, activating hardware DAZ and FTZ flushing.
    /// Alias for [`flush_to_zero`].
    #[inline]
    pub fn flush_to_zero() -> Self {
        Self::new()
    }

    /// Checks if hardware denormals_are_zero (DAZ) and flush_to_zero (FTZ) are currently active.
    #[inline]
    pub fn denormals_are_zero() -> bool {
        Self::is_active()
    }

    /// Explicitly consumes the guard and restores the previous floating-point control register state.
    #[inline]
    pub fn restore(self) {
        drop(self);
    }
}

impl Default for DenormalGuard {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DenormalGuard {
    #[inline]
    fn drop(&mut self) {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // SAFETY: Restoring the recorded prior MXCSR re-establishes the thread's exact previous floating-point control state.
            unsafe { write_mxcsr(self.prev_mxcsr) };
        }
    }
}

/// Executes a closure within an RAII denormal-flushed hardware scope.
///
/// Ensures both DAZ and FTZ are asserted for the duration of `f`, and guarantees
/// restoration of the CPU register state upon closure return or unwind.
#[inline]
pub fn denormal_flush_scope<R, F: FnOnce() -> R>(f: F) -> R {
    let _guard = DenormalGuard::new();
    f()
}

/// Executes a closure with hardware denormals flushed to zero.
/// Ergonomic alias for [`denormal_flush_scope`].
#[inline]
pub fn with_denormals_flushed<R, F: FnOnce() -> R>(f: F) -> R {
    denormal_flush_scope(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_denormal_guard_lifecycle() {
        // Record baseline state
        let initially_active = DenormalGuard::is_active();

        {
            let guard = DenormalGuard::new();
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            assert!(DenormalGuard::is_active());
            drop(guard);
        }

        // Must return to whatever state it was prior to guard instantiation
        assert_eq!(DenormalGuard::is_active(), initially_active);
    }

    #[test]
    fn test_denormal_flush_scope() {
        let initially_active = DenormalGuard::is_active();

        let result = denormal_flush_scope(|| {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            assert!(DenormalGuard::is_active());
            42u64
        });

        assert_eq!(result, 42);
        assert_eq!(DenormalGuard::is_active(), initially_active);
    }

    #[test]
    fn test_denormal_nested_scopes() {
        let initially_active = DenormalGuard::is_active();

        let result = denormal_flush_scope(|| {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            assert!(DenormalGuard::is_active());

            let inner = denormal_flush_scope(|| {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                assert!(DenormalGuard::is_active());
                100u32
            });

            assert_eq!(inner, 100);
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            assert!(DenormalGuard::is_active());

            inner * 2
        });

        assert_eq!(result, 200);
        assert_eq!(DenormalGuard::is_active(), initially_active);
    }

    #[test]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn test_subnormal_flush_computation() {
        // Construct a subnormal f32 (smallest positive normal f32 is ~1.17549435e-38)
        let subnormal = 1.0e-40f32;
        // Verify it is not zero IEEE-754 representation
        assert_ne!(subnormal.to_bits(), 0);

        let flushed_val = denormal_flush_scope(|| {
            // Under DAZ/FTZ, multiplying or operating on subnormal flushes to 0.0
            subnormal * 0.5f32
        });

        assert_eq!(flushed_val, 0.0f32);
    }

    #[test]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn test_restore_method() {
        let initially_active = DenormalGuard::is_active();
        let guard = DenormalGuard::new();
        assert!(DenormalGuard::is_active());
        guard.restore();
        assert_eq!(DenormalGuard::is_active(), initially_active);
    }

    #[test]
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    fn test_non_x86_fallback() {
        let guard = DenormalGuard::new();
        assert!(!DenormalGuard::is_active());
        drop(guard);
    }
}
