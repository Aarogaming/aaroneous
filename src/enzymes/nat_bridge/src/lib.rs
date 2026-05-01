/// NAT Bridge Enzyme
///
/// Connects the Aaroneous hive to a NATS server, routing processed data
/// from the AasBuffer into the federation event bus.
///
/// The enzyme receives a UTF-8 payload in the AasBuffer, parses it as the
/// current specialist output, and publishes it to `federation.enzyme.events`.
/// This bridges the legacy enzyme pipeline into the NATS federation topology.

use std::ffi::c_void;
use std::slice;
use std::sync::OnceLock;

#[repr(C)]
pub struct AasBuffer {
    data: *mut c_void,
    size: u64,
    capacity: u64,
}

/// Global NATS connection (lazily initialized on first use)
static NATS_URL: OnceLock<String> = OnceLock::new();

/// Topic to publish enzyme output events on
const ENZYME_EVENTS_TOPIC: &str = "federation.enzyme.events";

#[no_mangle]
pub extern "C" fn aas_init() -> i32 {
    // Read NATS URL from environment or use default
    let url = std::env::var("NATS_URL")
        .unwrap_or_else(|_| "nats://localhost:4222".to_string());
    NATS_URL.get_or_init(|| url.clone());
    println!("[nat_bridge] Initialized. NATS target: {}", url);
    0
}

#[no_mangle]
pub extern "C" fn aas_process(input: *mut AasBuffer, _output: *mut AasBuffer) -> i32 {
    unsafe {
        if input.is_null() || (*input).data.is_null() {
            return 2;
        }

        let curr_size = (*input).size as usize;
        if curr_size == 0 {
            return 0; // Nothing to publish
        }

        let slice = slice::from_raw_parts((*input).data as *const u8, curr_size);
        let payload_str = match std::str::from_utf8(slice) {
            Ok(s) => s.to_string(),
            Err(_) => return 3, // Invalid UTF-8
        };

        // Try to publish to NATS (best-effort: errors are logged, not fatal)
        let url = NATS_URL.get().map(|s| s.as_str()).unwrap_or("nats://localhost:4222");
        match nats::connect(url) {
            Ok(nc) => {
                let event = serde_json::json!({
                    "enzyme": "nat_bridge",
                    "payload": payload_str,
                    "timestamp_ms": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0),
                });
                let event_str = serde_json::to_string(&event).unwrap_or_default();
                if let Err(e) = nc.publish(ENZYME_EVENTS_TOPIC, event_str.as_bytes()) {
                    eprintln!("[nat_bridge] NATS publish error: {}", e);
                }
                let _ = nc.flush();
            }
            Err(e) => {
                // NATS not available — append a note to the buffer instead
                let msg = format!(" -> NAT Bridge: NATS unavailable ({})", e);
                let msg_bytes = msg.as_bytes();
                let capacity = (*input).capacity as usize;
                if curr_size + msg_bytes.len() <= capacity {
                    let write_slice = slice::from_raw_parts_mut(
                        (*input).data as *mut u8,
                        capacity,
                    );
                    std::ptr::copy_nonoverlapping(
                        msg_bytes.as_ptr(),
                        write_slice.as_mut_ptr().add(curr_size),
                        msg_bytes.len(),
                    );
                    (*input).size += msg_bytes.len() as u64;
                }
            }
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn aas_shutdown() -> i32 {
    println!("[nat_bridge] Shutdown complete.");
    0
}
