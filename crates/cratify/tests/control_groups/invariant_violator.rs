//! Invariant Violator — deliberately contaminated ACC for audit stress testing.
//!
//! This file is a synthetic control group that intentionally triggers all 7
//! Cratify audit rules. The audit engine must detect every violation.

// VIOLATION: forbidden dependency (unmanaged network stack)
use hyper::Client;

/// VIOLATION: unsafe block in function body.
pub fn do_unsafe_thing() {
    unsafe {
        let ptr = 0x1 as *mut u32;
        *ptr = 42;
    }
}

/// VIOLATION: contains todo!() panic macro.
pub fn unfinished_feature() {
    todo!("implement this later")
}

/// VIOLATION: unreachable!() panic macro.
pub fn impossible_path() {
    unreachable!("this should never happen")
}

/// VIOLATION: println! macro (should use tracing).
pub fn noisy_function() {
    println!("debug output here");
    eprintln!("error output here");
}

/// VIOLATION: public struct with only scalar fields — missing bytemuck::Pod.
pub struct NaiveHeader {
    pub tag: u32,
    pub len: u32,
}

/// VIOLATION: function name contains 'unsafe'.
pub fn unsafe_helper() {
    println!("helping unsafely");
}

/// VIOLATION: function accepts Option param (unwrap risk flagged).
pub fn maybe_process(data: Option<Vec<u8>>) {
    if let Some(d) = data {
        println!("got {} bytes", d.len());
    }
}

/// VIOLATION: unimplemented!() panic macro.
pub fn stub_fn() {
    unimplemented!("waiting for spec")
}

/// VIOLATION: function body uses .unwrap() on a Result.
pub fn risky_parse() {
    let val = "not_a_number".parse::<u32>().unwrap();
    println!("{val}");
}
