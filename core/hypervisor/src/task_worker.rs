use anyhow::{Result, anyhow};
use ipc_bus::shared_memory::McpToolCallFrame;
use serde_json::Value;

pub struct ExecutionEnzyme;

impl ExecutionEnzyme {
    /// Simulates a multi-tool chain execution within the WASM sandbox.
    pub fn execute_chain(plan_json: &str, frame: &mut McpToolCallFrame) -> Result<()> {
        println!("[ExecutionEnzyme] Initializing multi-tool execution chain...");

        let plan: Value = serde_json::from_str(plan_json)?;
        let steps = plan["steps"]
            .as_array()
            .ok_or_else(|| anyhow!("Invalid execution plan: missing steps"))?;

        for step in steps {
            let tool_name = step["tool"]
                .as_str()
                .ok_or_else(|| anyhow!("Step missing tool name"))?;
            let args = &step["arguments"];

            println!(
                "[ExecutionEnzyme] Executing step: {} with args: {}",
                tool_name, args
            );

            // In a real WASM environment, this would call host-provided functions
            // mapped to the SignalBridge mmap.
            Self::simulate_host_call(tool_name, args, frame)?;
        }

        println!("[ExecutionEnzyme] Chain execution complete.");
        Ok(())
    }

    fn simulate_host_call(name: &str, args: &Value, _frame: &mut McpToolCallFrame) -> Result<()> {
        match name {
            "read_file" => {
                let path = args["path"].as_str().unwrap_or("");
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        let len = content.len();
                        println!("  +3 Host Result: File content loaded ({} bytes).", len);
                        Ok(())
                    }
                    Err(e) => Err(anyhow!("Failed to read file {}: {}", path, e)),
                }
            }
            "write_file" => {
                let path = args["path"].as_str().unwrap_or("");
                let content = args["content"].as_str().unwrap_or("");
                match std::fs::write(path, content) {
                    Ok(_) => {
                        println!("  +3 Host Result: Buffer flushed to disk ({}).", path);
                        Ok(())
                    }
                    Err(e) => Err(anyhow!("Failed to write file {}: {}", path, e)),
                }
            }
            "http_request" => {
                let url = args["url"].as_str().unwrap_or("");
                if url.starts_with("http") {
                    println!("  +3 Host Result: HTTP Request queued for {}.", url);
                    Ok(())
                } else {
                    Err(anyhow!("Invalid URL for HTTP request: {}", url))
                }
            }
            _ => Err(anyhow!("Unknown tool: {}", name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_execute_chain_success() {
        let dir = tempdir().expect("tempdir");
        let test_file = dir.path().join("test.txt");
        let test_file_str = test_file.to_str().unwrap().replace('\\', "/");

        let plan = serde_json::json!({
            "steps": [
                {
                    "tool": "write_file",
                    "arguments": {
                        "path": test_file_str,
                        "content": "hello world"
                    }
                },
                {
                    "tool": "read_file",
                    "arguments": {
                        "path": test_file_str
                    }
                },
                {
                    "tool": "http_request",
                    "arguments": {
                        "url": "https://localhost:8080/health"
                    }
                }
            ]
        });

        let mut frame = McpToolCallFrame::default();
        let res = ExecutionEnzyme::execute_chain(&plan.to_string(), &mut frame);
        assert!(res.is_ok());
    }

    #[test]
    fn test_execute_chain_invalid_tool() {
        let plan = serde_json::json!({
            "steps": [
                {
                    "tool": "nonexistent_tool",
                    "arguments": {}
                }
            ]
        });

        let mut frame = McpToolCallFrame::default();
        let res = ExecutionEnzyme::execute_chain(&plan.to_string(), &mut frame);
        assert!(res.is_err());
    }
}
