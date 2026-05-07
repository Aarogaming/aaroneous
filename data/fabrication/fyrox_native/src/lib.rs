use std::ffi::c_void;
use serde_json::{json, Value};

#[repr(C)]
pub struct AasBuffer {
    pub data: *mut c_void,
    pub size: u64,
    pub capacity: u64,
}

#[no_mangle]
pub extern "C" fn aas_init() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn aas_process(_input: *mut AasBuffer, _output: *mut AasBuffer) -> i32 {
    0
}