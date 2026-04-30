use std::ffi::c_void;
use std::slice;

#[repr(C)]
pub struct AasBuffer {
    data: *mut c_void,
    size: u64,
    capacity: u64,
}

#[no_mangle]
pub extern "C" fn aas_init() -> i32 {
    println!("[thought_kernel] Initialization complete.");
    0 // AAS_OK
}

#[no_mangle]
pub extern "C" fn aas_process(input: *mut AasBuffer, _output: *mut AasBuffer) -> i32 {
    unsafe {
        if (*input).data.is_null() {
            return 2; // AAS_ERROR_MEMORY_ACCESS
        }
        
        let slice = slice::from_raw_parts_mut((*input).data as *mut u8, (*input).capacity as usize);
        let msg = b" -> Thought Kernel Processed";
        let curr_size = (*input).size as usize;
        
        if curr_size + msg.len() <= slice.len() {
            std::ptr::copy_nonoverlapping(msg.as_ptr(), slice.as_mut_ptr().add(curr_size), msg.len());
            (*input).size += msg.len() as u64;
        }
    }
    0 // AAS_OK
}

#[no_mangle]
pub extern "C" fn aas_shutdown() -> i32 {
    println!("[thought_kernel] Shutdown complete.");
    0 // AAS_OK
}
