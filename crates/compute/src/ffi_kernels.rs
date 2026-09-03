// crates/compute/src/ffi_kernels.rs
use std::alloc::{alloc, Layout};

/// Basic CPU fallback for matrix multiplication.
///
/// # Safety
/// Caller must ensure `a_ptr`, `b_ptr`, and `out_ptr` point to valid memory buffers
/// of lengths `m * k`, `k * n`, and `m * n` respectively.
#[no_mangle]
pub unsafe extern "C" fn host_tensor_dot_kernel(
    a_ptr: *const f32,
    b_ptr: *const f32,
    out_ptr: *mut f32,
    m: usize,
    n: usize,
    k: usize,
) {
    let a = std::slice::from_raw_parts(a_ptr, m * k);
    let b = std::slice::from_raw_parts(b_ptr, k * n);
    let out = std::slice::from_raw_parts_mut(out_ptr, m * n);

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
/// Caller must ensure `size` fits within available host memory and subsequent accesses
/// obey standard alignment invariants.
#[no_mangle]
pub unsafe extern "C" fn host_alloc(size: usize, align: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    let safe_align = if align == 0 || !align.is_power_of_two() { 64 } else { align };
    let layout = Layout::from_size_align_unchecked(size, safe_align);
    alloc(layout)
}

/// Host-side CPU entropy minimization / state dissipation fallback kernel.
///
/// # Safety
/// Caller must ensure `state_ptr` is either null or points to an array of at least `len` floats.
#[no_mangle]
pub unsafe extern "C" fn host_entropy_min(state_ptr: *mut f32, len: usize) {
    if state_ptr.is_null() || len == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(state_ptr, len);
    for v in slice.iter_mut() {
        *v *= 0.5;
    }
}
