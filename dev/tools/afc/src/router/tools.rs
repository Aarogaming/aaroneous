// dev/tools/afc/src/router/tools.rs
use crate::router::types::{FunctionDefinition, ToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchProposal {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub target_content: String,
    pub replacement_content: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditDefectItem {
    pub task_id: String,
    pub file_path: String,
    pub line_number: Option<usize>,
    pub tier: String,
    pub defect_type: String,
    pub description: String,
}

pub struct ToolRegistry;

impl ToolRegistry {
    /// Return standard tool definition for propose_patch
    pub fn propose_patch_tool() -> ToolDefinition {
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "propose_patch".to_string(),
                description:
                    "Propose a precise, contiguous code replacement in a specified target file."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Relative path to target file (e.g. 'crates/compute/src/engine.rs')"
                        },
                        "start_line": {
                            "type": "integer",
                            "description": "Starting line number (1-indexed, inclusive)"
                        },
                        "end_line": {
                            "type": "integer",
                            "description": "Ending line number (1-indexed, inclusive)"
                        },
                        "target_content": {
                            "type": "string",
                            "description": "Exact text chunk to be replaced (must match existing file content verbatim)"
                        },
                        "replacement_content": {
                            "type": "string",
                            "description": "Clean, Result-bubbled Rust replacement code without panics or unsafe"
                        },
                        "explanation": {
                            "type": "string",
                            "description": "Brief architectural explanation of the remediation"
                        }
                    },
                    "required": ["file_path", "start_line", "end_line", "target_content", "replacement_content"],
                    "additionalProperties": false
                }),
            },
        }
    }

    /// Return standard tool definition for reporting novel audit defects
    pub fn report_defect_tool() -> ToolDefinition {
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "report_defect".to_string(),
                description: "Report a novel defect to append to the active audit queue."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "Category ID prefix (e.g. 'TECH-01', 'SEC-04', 'PERF-02')"
                        },
                        "file_path": {
                            "type": "string",
                            "description": "Path to affected file"
                        },
                        "line_number": {
                            "type": "integer",
                            "description": "Approximate line number of defect"
                        },
                        "tier": {
                            "type": "string",
                            "enum": ["T1", "T2", "T3"],
                            "description": "Severity tier (T1: Critical/Panic, T2: Major/Smell, T3: Minor/Debt)"
                        },
                        "defect_type": {
                            "type": "string",
                            "description": "Type of defect (e.g. 'Unwrap Removal', 'Dead Code', 'Memory Leak')"
                        },
                        "description": {
                            "type": "string",
                            "description": "Detailed forensic description and recommended fix"
                        }
                    },
                    "required": ["task_id", "file_path", "tier", "defect_type", "description"],
                    "additionalProperties": false
                }),
            },
        }
    }
}
