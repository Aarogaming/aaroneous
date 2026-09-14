// crates/compute/src/ffi_kernels.rs
use std::alloc::{alloc, Layout};

/// Basic CPU fallback for matrix multiplication.
///
/// # Safety
/// - `a_ptr`, `b_ptr`, and `out_ptr` point to valid memory buffers of lengths `m * k`, `k * n`, and `m * n` respectively
/// - Pointers are properly aligned per `f32` requirements (4-byte alignment)
/// - No overlap between input/output buffers violates aliasing assumptions
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_tensor_dot_kernel(
    a_ptr: *const f32,
    b_ptr: *const f32,
    out_ptr: *mut f32,
    m: usize,
    n: usize,
    k: usize,
) {
    // SAFETY: Caller guarantees valid, aligned pointers per function contract
    let a = unsafe { std::slice::from_raw_parts(a_ptr, m * k) };
    let b = unsafe { std::slice::from_raw_parts(b_ptr, k * n) };
    let out = unsafe { std::slice::from_raw_parts_mut(out_ptr, m * n) };

    for row in 0..m {
        for col in 0..n {
            let mut sum: f32 = 0.0;
            for step in 0..k {
                sum += a[row * k + step] * b[step * n + col];
            }
            out[row * n + col] = sum;
        }
    }
}

/// Host-side memory allocator for JIT graphs.
///
/// # Safety
/// - `size` fits within available host memory
/// - `align` is power-of-two; default to 64-byte alignment if invalid
/// - Returns null pointer for zero-size requests per FFI contract
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_alloc(size: usize, align: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    let safe_align = if align == 0 || !align.is_power_of_two() { 64 } else { align };
    // SAFETY: Layout constructed with validated size/alignment; caller owns returned memory
    let layout = unsafe { Layout::from_size_align_unchecked(size, safe_align) };
    unsafe { alloc(layout) }
}

/// Host-side CPU entropy minimization / state dissipation fallback kernel.
///
/// # Safety
/// - `state_ptr` is either null or points to a valid array of at least `len` floats
/// - `len` accurately reflects the number of elements in target buffer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_entropy_min(state_ptr: *mut f32, len: usize) {
    if state_ptr.is_null() || len == 0 {
        return;
    }
    // SAFETY: Caller guarantees valid pointer and length per function contract
    let slice = unsafe { std::slice::from_raw_parts_mut(state_ptr, len) };
    for v in slice.iter_mut() {
        *v *= 0.5;
    }
}
