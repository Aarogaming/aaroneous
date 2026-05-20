use anyhow::{Result, anyhow};
use serde_json::Value;
use crate::nervous_system::shared_memory::McpToolCallFrame;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct McpGateway;

impl McpGateway {
    /// Translates an incoming JSON-RPC tool call into a zero-copy binary frame.
    pub fn translate_inbound(json_rpc: &str, frame: &mut McpToolCallFrame) -> Result<()> {
        let v: Value = serde_json::from_str(json_rpc)?;
        
        let tool_name = v["params"]["name"].as_str()
            .ok_or_else(|| anyhow!("Invalid MCP request: missing tool name"))?;
            
        let arguments = v["params"]["arguments"].to_string();
        let arg_bytes = arguments.as_bytes();
        
        if arg_bytes.len() > 2048 {
            return Err(anyhow!("MCP arguments exceed mmap buffer size (2048 bytes)"));
        }

        // Calculate name hash
        let mut hasher = DefaultHasher::new();
        tool_name.hash(&mut hasher);
        
        frame.call_id += 1;
        frame.tool_name_hash = hasher.finish();
        frame.status = 1; // Pending
        frame.arguments_size = arg_bytes.len() as u32;
        frame.arguments_payload[..arg_bytes.len()].copy_from_slice(arg_bytes);

        println!("[McpGateway] Inbound translation complete for tool: {}", tool_name);
        Ok(())
    }

    /// Translates the binary frame result back into a JSON-RPC response.
    pub fn translate_outbound(frame: &McpToolCallFrame) -> Result<String> {
        if frame.status != 3 && frame.status != 4 {
            return Err(anyhow!("Tool execution not complete (Status: {})", frame.status));
        }

        let result_str = std::str::from_utf8(&frame.arguments_payload[..frame.arguments_size as usize])
            .map_err(|e| anyhow!("Failed to decode MCP result: {}", e))?;

        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": frame.call_id,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": result_str
                    }
                ]
            }
        });

        Ok(response.to_string())
    }
}
