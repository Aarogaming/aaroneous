use serde_json::{json, Value};
use std::ffi::{CStr, CString};

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn describe() -> *mut std::os::raw::c_char {
    let capabilities = json!({
        "name": "template_plugin",
        "description": "A template plugin that wraps an open-source crate.",
        "commands": {
            "execute": {
                "description": "Execute task",
                "inputs": {"payload": "string"},
                "outputs": {"status": "string"}
            }
        }
    });
    let s = CString::new(capabilities.to_string()).unwrap();
    s.into_raw()
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn execute_task(
    cmd_ptr: *const std::os::raw::c_char,
    payload_ptr: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let _cmd = unsafe { CStr::from_ptr(cmd_ptr).to_string_lossy().into_owned() };
    let payload = unsafe { CStr::from_ptr(payload_ptr).to_string_lossy().into_owned() };

    let args: Value = serde_json::from_str(&payload).unwrap_or(json!({}));

    // [TARGET_CRATE_ROUTING]
    // The Genesis Architect will inject the routing logic here.
    let result = json!({"status": "success", "received": args}).to_string();

    CString::new(result).unwrap().into_raw()
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn free_string(s: *mut std::os::raw::c_char) {
    unsafe {
        if s.is_null() {
            return;
        }
        let _ = CString::from_raw(s);
    }
}
