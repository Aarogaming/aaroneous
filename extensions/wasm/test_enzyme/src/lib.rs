use std::ffi::CString;
use std::os::raw::c_char;

// Import the host function provided by the Hypervisor
extern "C" {
    fn synapse_write(offset: u32, ptr: *const c_char, len: u32) -> i32;
}

#[no_mangle]
pub extern "C" fn run() -> i32 {
    // Write "ENZYME_ACTIVE" to the Synapse at offset 300
    let msg = CString::new("ENZYME_ACTIVE").unwrap();
    let ptr = msg.as_ptr();
    let len = msg.as_bytes().len() as u32;

    unsafe {
        let result = synapse_write(300, ptr, len);
        if result == 0 {
            1 // Success
        } else {
            result // Error code
        }
    }
}
