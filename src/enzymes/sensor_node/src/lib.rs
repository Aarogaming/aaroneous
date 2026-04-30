use sysinfo::{System};
use std::ffi::c_void;

#[repr(C)]
pub struct AasBuffer {
    data: *mut c_void,
    size: u64,
    capacity: u64,
}

#[no_mangle]
pub extern "C" fn aas_init() -> i32 {
    println!("[sensor_node] Sensory filaments extended.");
    0
}

#[no_mangle]
pub extern "C" fn aas_process(input: *mut AasBuffer, _output: *mut AasBuffer) -> i32 {
    unsafe {
        if input.is_null() || (*input).data.is_null() { return 2; }
        
        let mut sys = System::new_all();
        sys.refresh_cpu();
        
        let load = sys.global_cpu_info().cpu_usage();
        let msg = format!(" -> SensorNode: CPU Load {:.1}%", load);
        let slice = std::slice::from_raw_parts_mut((*input).data as *mut u8, (*input).capacity as usize);
        let curr_size = (*input).size as usize;
        
        if curr_size + msg.len() <= slice.len() {
            std::ptr::copy_nonoverlapping(msg.as_ptr(), slice.as_mut_ptr().add(curr_size), msg.len());
            (*input).size += msg.len() as u64;
        }
    }
    0
}
