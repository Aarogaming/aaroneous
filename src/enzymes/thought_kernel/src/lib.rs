/// Thought Kernel Enzyme
///
/// Processes text from the AasBuffer through the configured language model.
/// The enzyme reads the current buffer content as a prompt, queries the LLM,
/// and appends the response back to the buffer.
///
/// # Configuration (environment variables)
///
/// - `THOUGHT_KERNEL_MODEL`: path to the GGUF model file (required for real inference)
/// - `THOUGHT_KERNEL_MAX_TOKENS`: max tokens to generate (default: 128)
/// - `THOUGHT_KERNEL_TEMP`: temperature (default: 0.7)
///
/// When `THOUGHT_KERNEL_MODEL` is not set, the enzyme appends a placeholder
/// string so the pipeline continues without blocking.

use std::ffi::c_void;
use std::slice;
use std::sync::OnceLock;

#[repr(C)]
pub struct AasBuffer {
    data: *mut c_void,
    size: u64,
    capacity: u64,
}

/// Cached model path from environment
static MODEL_PATH: OnceLock<Option<String>> = OnceLock::new();

#[no_mangle]
pub extern "C" fn aas_init() -> i32 {
    let model_path = std::env::var("THOUGHT_KERNEL_MODEL").ok();
    match &model_path {
        Some(p) => println!("[thought_kernel] Model: {}", p),
        None => println!("[thought_kernel] No model configured (set THOUGHT_KERNEL_MODEL for inference)"),
    }
    MODEL_PATH.get_or_init(|| model_path);
    0
}

#[no_mangle]
pub extern "C" fn aas_process(input: *mut AasBuffer, _output: *mut AasBuffer) -> i32 {
    unsafe {
        if input.is_null() || (*input).data.is_null() {
            return 2;
        }

        let curr_size = (*input).size as usize;
        let capacity = (*input).capacity as usize;

        if curr_size == 0 {
            return 0;
        }

        let prompt_bytes = slice::from_raw_parts((*input).data as *const u8, curr_size);
        let prompt = match std::str::from_utf8(prompt_bytes) {
            Ok(s) => s.to_string(),
            Err(_) => return 3,
        };

        // Try real inference if a model is configured
        let response = match MODEL_PATH.get().and_then(|p| p.as_ref()) {
            Some(model_path) => {
                // Use the llama-gguf CLI (if installed) via subprocess.
                // This is the simplest approach for a DLL enzyme that can't
                // link against the main crate's llama-gguf integration.
                let max_tokens = std::env::var("THOUGHT_KERNEL_MAX_TOKENS")
                    .ok()
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(128);

                // Check if llama-gguf CLI is available
                match std::process::Command::new("llama-gguf")
                    .args([
                        "--model", model_path,
                        "--prompt", &prompt,
                        "--max-tokens", &max_tokens.to_string(),
                        "--format", "text",
                    ])
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        String::from_utf8_lossy(&output.stdout).trim().to_string()
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        format!(" -> Thought Kernel: inference failed ({})", stderr.trim())
                    }
                    Err(e) => {
                        // llama-gguf CLI not found — fall back to placeholder
                        format!(" -> Thought Kernel: llama-gguf CLI unavailable ({})", e)
                    }
                }
            }
            None => {
                // No model configured — use heuristic response
                let word_count = prompt.split_whitespace().count();
                format!(
                    " -> Thought Kernel: processed {} tokens (no model — set THOUGHT_KERNEL_MODEL)",
                    word_count
                )
            }
        };

        // Write response back into the buffer
        let response_bytes = response.as_bytes();
        let write_slice = slice::from_raw_parts_mut((*input).data as *mut u8, capacity);
        if curr_size + response_bytes.len() <= capacity {
            std::ptr::copy_nonoverlapping(
                response_bytes.as_ptr(),
                write_slice.as_mut_ptr().add(curr_size),
                response_bytes.len(),
            );
            (*input).size += response_bytes.len() as u64;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn aas_shutdown() -> i32 {
    println!("[thought_kernel] Shutdown complete.");
    0
}
