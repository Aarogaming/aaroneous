// crates/compute/src/ffi_kernels.rs
use std::alloc::{alloc, Layout};

/// Basic CPU fallback for matrix multiplication.
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
            let mut sum = 0.0;
            for step in 0..k { sum += a[row * k + step] * b[step * n + col]; }
            out[row * n + col] = sum as f32;}
    }
}

/// Host-side memory allocator for JIT graphs.
#[no_mangle]
pub unsafe extern "C" fn host_alloc(size: usize, align: usize) -> *mut u8 {
    if size == 0 { return std::ptr::null_mut(); }
    let safe_align = if align == 0 || !align.is_power_of_two() { 64 } else { align };
    let layout = Layout::from_size_align_unchecked(size, safe_align);
    alloc(layout)
}

/// Host-side CPU entropy minimization / state dissipation fallback kernel.
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
