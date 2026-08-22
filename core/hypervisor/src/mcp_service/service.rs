use serde::{Deserialize, Serialize};
use std::collections::HashMap;
/// Aaroneous MCP Service — Anthropic Model Context Protocol server.
///
/// Implements the MCP 2024-11 specification (JSON-RPC 2.0 over HTTP+SSE).
/// This makes every sovereign specialist available as an MCP tool to:
///   - Claude Desktop (Settings → Developer → MCP Servers)
///   - Cursor IDE (Settings → Features → Model Context Protocol)
///   - VS Code with Copilot (via MCP extension)
///   - Any MCP-compatible client
///
/// Wire format: JSON-RPC 2.0
/// Transport: HTTP POST (requests) + GET SSE (server-initiated notifications)
/// Port: 8766 (separate from the REST API on 8765)
///
/// # Tool mapping
///
/// Each sovereign becomes an MCP tool:
///
/// | Tool name      | Description                              | Input schema             |
/// |----------------|------------------------------------------|--------------------------|
/// | ask_merlin     | Research and knowledge synthesis         | {query: string}          |
/// | ask_odin       | Task decomposition and planning          | {intent: string}         |
/// | ask_ariel      | UI/UX design generation                  | {intent: string}         |
/// | ask_argus      | Security audit and vulnerability scan    | {target: string}         |
/// | ask_wen        | Human state classification from context  | {context: string}        |
/// | ask_hephaestus | Build and fabrication planning           | {task: string}           |
/// | submit_intent  | Submit to the full sovereign hive        | {content: string}        |
/// | get_results    | Retrieve recent execution results        | {}                       |
/// | get_specialists| List all active specialists and state    | {}                       |
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::federation::hive::Federation;
use crate::mcp_service::{CapabilityDomain, ServiceConfig};

// ── JSON-RPC 2.0 types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    pub fn ok(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }
    pub fn err(id: Option<serde_json::Value>, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

// ── MCP Tool definitions ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

impl McpTool {
    pub fn new(
        name: &str,
        description: &str,
        props: serde_json::Value,
        required: Vec<&str>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": props,
                "required": required,
            }),
        }
    }
}

// ── McpService ────────────────────────────────────────────────────────────────

pub struct McpService {
    pub config: ServiceConfig,
    pub federation: Option<Arc<Federation>>,
    pub tools: Arc<RwLock<Vec<McpTool>>>,
    pub domains: Arc<RwLock<HashMap<String, CapabilityDomain>>>,
    pub started_at: std::time::Instant,
    pub request_count: Arc<std::sync::atomic::AtomicU64>,
    /// Per-session conversation context for multi-turn MCP tool calls.
    pub sessions: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Workspace root for file tools (read_code, search_code, list_files).
    ///
    /// Resolution order:
    /// 1. `AARONEOUS_WORKSPACE` environment variable
    /// 2. `std::env::current_dir()` (process working directory)
    /// 3. Hardcoded `D:\Aaroneous` fallback (only for self-development)
    ///
    /// Claude Desktop / Cursor: set `AARONEOUS_WORKSPACE=${workspaceFolder}`
    /// in the MCP server environment config.
    pub workspace_root: std::path::PathBuf,
}

impl McpService {
    pub fn new(config: ServiceConfig) -> Self {
        // Discover workspace root at startup — dynamically resolved via aaroneous_paths
        let workspace_root = aaroneous_paths::WorkspacePaths::discover().root().clone();

        tracing::info!("MCP workspace root: {}", workspace_root.display());

        Self {
            config,
            federation: None,
            tools: Arc::new(RwLock::new(Vec::new())),
            domains: Arc::new(RwLock::new(HashMap::new())),
            started_at: std::time::Instant::now(),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            workspace_root,
        }
    }

    /// Attach the live federation so tools can call sovereigns.
    pub fn with_federation(mut self, federation: Arc<Federation>) -> Self {
        self.federation = Some(federation);
        self
    }

    /// Report LLM provider status for the /health endpoint.
    pub fn llm_provider_status(&self) -> serde_json::Value {
        let gguf_compiled = cfg!(feature = "llama-gguf");
        serde_json::json!({
            "llama_gguf_feature": gguf_compiled,
            "inference_available": gguf_compiled,
        })
    }

    /// Register all sovereign specialists as MCP tools.
    ///
    /// Called once at startup after federation is attached.
    pub async fn register_sovereign_tools(&self) {
        let mut tools = self.tools.write().await;
        tools.clear();

        // Core tools — one per sovereign
        tools.push(McpTool::new("ask_merlin",
            "Research, knowledge synthesis, and factual queries. \
             Merlin bridges Aaroneous to external knowledge. \
             Use for: research summaries, fact-checking, topic synthesis.",
            serde_json::json!({ "query": { "type": "string", "description": "Research query or topic to investigate" } }),
            vec!["query"],
        ));
        tools.push(McpTool::new("ask_odin",
            "Task decomposition and guild coordination. \
             Odin breaks complex intents into sovereign task graphs. \
             Use for: planning, dependency identification, multi-step workflows.",
            serde_json::json!({ "intent": { "type": "string", "description": "Complex intent to decompose into tasks" } }),
            vec!["intent"],
        ));
        tools.push(McpTool::new("ask_ariel",
            "UI/UX design generation and Maelstrom spatial visualization. \
             Ariel produces structured design variants with colors, typography, layouts. \
             Use for: interface design, visual hierarchy, design system decisions.",
            serde_json::json!({ "intent": { "type": "string", "description": "Design intent or UI requirement" } }),
            vec!["intent"],
        ));
        tools.push(McpTool::new("ask_argus",
            "Security audit, vulnerability scanning, and adversarial analysis. \
             Argus sees all threats. Abliterated — no refusals on security topics. \
             Use for: code audits, CVE analysis, threat modelling, secrets scanning.",
            serde_json::json!({ "target": { "type": "string", "description": "Code, system, or configuration to audit" } }),
            vec!["target"],
        ));
        tools.push(McpTool::new("ask_wen",
            "Human state classification and biometric-adaptive responses. \
             Wen reads context and adapts hive behavior to the human's current capacity. \
             Use for: stress detection, focus assessment, interruption policy.",
            serde_json::json!({ "context": { "type": "string", "description": "Human state context or biometric readings" } }),
            vec!["context"],
        ));
        tools.push(McpTool::new("ask_hephaestus",
            "Build automation, fabrication planning, and infrastructure maintenance. \
             Hephaestus keeps the forge running. \
             Use for: build scripts, deployment plans, dependency management, CI/CD.",
            serde_json::json!({ "task": { "type": "string", "description": "Build or fabrication task to plan" } }),
            vec!["task"],
        ));
        tools.push(McpTool::new("signal_wasms",
            "Emit a signal to all active WASM agents in the Aaroneous runtime. \
             Use this to coordinate between Python shards and low-level agent logic.",
            serde_json::json!({
                "signal_type": { "type": "string", "description": "Discriminator for the signal (e.g. 'RECALIBRATE')" },
                "payload": { "type": "object", "description": "JSON payload to pass to WASM perception" }
            }),
            vec!["signal_type", "payload"],
        ));
        tools.push(McpTool::new("memory_sync",
            "Synchronize or retrieve distributed memory entries for a specialist shard. \
             Use this to share context between Python Shards and the Rust Core.",
            serde_json::json!({
                "shard_name": { "type": "string", "description": "Name of the shard syncing memory" },
                "action": { "type": "string", "enum": ["push", "pull", "list"], "description": "Sync action to perform" },
                "entries": { "type": "array", "description": "Entries to push (for 'push' action)", "items": { "type": "object" } }
            }),
            vec!["shard_name", "action"],
        ));
        tools.push(McpTool::new(
            "federated_task_dispatch",
            "Dispatch a task to the federated guild. \
             Use this for complex orchestration tasks that require multi-sovereign coordination.",
            serde_json::json!({
                "task_id": { "type": "string", "description": "Unique task identifier" },
                "instruction": { "type": "string", "description": "High-level task instruction" },
                "priority": { "type": "string", "enum": ["high", "medium", "low"] }
            }),
            vec!["task_id", "instruction"],
        ));
        tools.push(McpTool::new("ask_hermes",
            "P2P mesh sync, CRDT conflict resolution, multi-device state coordination. \
             Hermes is always in motion — makes the hive feel like one thing. \
             Use for: sync conflicts, device coordination, state consistency.",
            serde_json::json!({ "scenario": { "type": "string", "description": "Sync scenario or conflict to resolve" } }),
            vec!["scenario"],
        ));
        tools.push(McpTool::new("ask_kami",
            "AR/VR spatial reasoning and physical/digital boundary management. \
             Kami materializes digital intent into physical space. \
             Use for: spatial anchor placement, 3D coordinate reasoning, AR overlays.",
            serde_json::json!({ "spatial_intent": { "type": "string", "description": "Spatial or AR/VR placement intent" } }),
            vec!["spatial_intent"],
        ));
        tools.push(McpTool::new("ask_dionysus",
            "DNA Bank archival, memory consolidation, and pattern extraction. \
             Dionysus remembers so the hive can learn. \
             Use for: session archival, pattern discovery, long-term memory retrieval.",
            serde_json::json!({ "content": { "type": "string", "description": "Content to archive or retrieve patterns from" } }),
            vec!["content"],
        ));

        // Hive-level tools
        tools.push(McpTool::new("submit_intent",
            "Submit an intent to the full Aaroneous sovereign hive. \
             All 9 specialists process the intent in parallel via Odin's coordination. \
             Use for: complex multi-domain tasks that need multiple specialists.",
            serde_json::json!({
                "content": { "type": "string", "description": "The intent or task for the full hive" },
                "priority": {
                    "type": "string",
                    "description": "Priority level",
                    "enum": ["Background", "Normal", "High", "Critical"],
                    "default": "Normal"
                }
            }),
            vec!["content"],
        ));
        tools.push(McpTool::new(
            "get_results",
            "Retrieve the most recent execution results from all sovereigns. \
             Returns the last 10 specialist outputs.",
            serde_json::json!({}),
            vec![],
        ));
        tools.push(McpTool::new(
            "get_specialists",
            "List all active sovereign specialists with their current confidence scores, \
             execution counts, and domain descriptions.",
            serde_json::json!({}),
            vec![],
        ));
        tools.push(McpTool::new("forge_hybrid",
            "Create a hybrid sovereign by DNA-splicing two models. \
             Uses the splice_boundary from DNA comparison to determine the optimal cut point.",
            serde_json::json!({
                "model_a": { "type": "string", "description": "First model filename (e.g. 'merlin-qwen2.5-7b.gguf')" },
                "model_b": { "type": "string", "description": "Second model filename" },
                "sovereign_name": { "type": "string", "description": "Name for the resulting hybrid sovereign" },
                "splice_boundary": { "type": "integer", "description": "Override splice point (default: auto from DNA)" }
            }),
            vec!["model_a", "model_b", "sovereign_name"],
        ));

        // Developer workflow tools
        tools.push(McpTool::new("read_code",
            "Read source code from a file path. Use this before ask_argus, ask_merlin, or \
             ask_hephaestus so the sovereign receives the actual code rather than a description. \
             Supports relative paths from the Aaroneous workspace or absolute paths.",
            serde_json::json!({
                "path": { "type": "string", "description": "File path to read (relative to workspace or absolute)" },
                "start_line": { "type": "integer", "description": "First line to read (1-indexed, default: 1)" },
                "end_line": { "type": "integer", "description": "Last line to read (default: 200)" },
            }),
            vec!["path"],
        ));
        tools.push(McpTool::new("search_code",
            "Search for a pattern across source files in the workspace. Returns matching lines \
             with file paths and line numbers. Use before ask_argus to find all usages of a \
             potentially vulnerable pattern.",
            serde_json::json!({
                "pattern": { "type": "string", "description": "Search pattern (supports basic regex)" },
                "path": { "type": "string", "description": "Directory to search (default: workspace root)" },
                "file_glob": { "type": "string", "description": "File pattern filter e.g. '*.rs' (default: all)" },
                "max_results": { "type": "integer", "description": "Maximum results to return (default: 20)" },
            }),
            vec!["pattern"],
        ));
        tools.push(McpTool::new("list_files",
            "List files in a directory. Use to explore the workspace structure before reading \
             specific files.",
            serde_json::json!({
                "path": { "type": "string", "description": "Directory path (default: workspace root)" },
                "glob": { "type": "string", "description": "File pattern e.g. '*.rs'" },
            }),
            vec![],
        ));

        // Meta-tool: assembles all recent sovereign outputs into a coherent report
        tools.push(McpTool::new(
            "hive_summary",
            "Get a structured summary of the most recent sovereign outputs, assembled \
             into a coherent markdown report. Shows what each specialist contributed, \
             their confidence, and key findings. Call after submit_intent to see \
             the full hive perspective in a readable format.",
            serde_json::json!({
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of recent results to include (default: 9)",
                    "default": 9,
                }
            }),
            vec![],
        ));

        tools.push(McpTool::new("register_shard",
            "Register an external AAS Shard as a first-class specialist. \
             Enables bidirectional tasking and token governance.",
            serde_json::json!({
                "name": { "type": "string", "description": "Unique name of the shard" },
                "capabilities": { "type": "array", "items": { "type": "string" }, "description": "Specialist domains" },
                "endpoint": { "type": "string", "description": "SSE endpoint for task delivery" }
            }),
            vec!["name", "capabilities"],
        ));
        tools.push(McpTool::new(
            "metabolic_heartbeat",
            "Submit a metabolic heartbeat from a Shard. Syncs VRAM/CPU usage.",
            serde_json::json!({
                "name": { "type": "string" },
                "vram_mb": { "type": "integer" },
                "cpu_pct": { "type": "number" },
                "token_request": { "type": "number", "default": 1.0 }
            }),
            vec!["name"],
        ));

        info!("Registered {} MCP tools", tools.len());
    }

    /// Handle an incoming JSON-RPC 2.0 request.
    ///
    /// Dispatches to the correct MCP method handler.
    pub async fn handle_jsonrpc(&self, raw: serde_json::Value) -> JsonRpcResponse {
        let id = raw.get("id").cloned();
        let method = match raw.get("method").and_then(|m| m.as_str()) {
            Some(m) => m.to_string(),
            None => return JsonRpcResponse::err(id, -32600, "Invalid Request: missing method"),
        };
        let params = raw
            .get("params")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        debug!("MCP request: method={}", method);

        match method.as_str() {
            // ── MCP lifecycle ──────────────────────────────────────────────
            "initialize" => JsonRpcResponse::ok(
                id,
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": { "listChanged": false },
                        "resources": {},
                        "prompts": {},
                        "logging": {}
                    },
                    "serverInfo": {
                        "name": "Aaroneous",
                        "version": env!("CARGO_PKG_VERSION"),
                        "description": "Sovereign AI hive — 9 specialized agents, GGUF-backed, abliterated bases"
                    }
                }),
            ),
            "notifications/initialized" => {
                // Client acknowledges initialize — no response needed for notifications
                JsonRpcResponse::ok(id, serde_json::Value::Null)
            }
            "ping" => JsonRpcResponse::ok(id, serde_json::json!({})),

            // ── Tools ───────────────────────────────────────────────────────
            "tools/list" => {
                let tools = self.tools.read().await;
                JsonRpcResponse::ok(id, serde_json::json!({ "tools": *tools }))
            }
            "tools/call" => self.handle_tool_call(id, params).await,

            // ── Resources (empty — no file resources exposed) ──────────────
            "resources/list" => JsonRpcResponse::ok(id, serde_json::json!({ "resources": [] })),
            "resources/read" => JsonRpcResponse::err(id, -32002, "No resources available"),

            // ── Prompts ─────────────────────────────────────────────────────
            "prompts/list" => JsonRpcResponse::ok(
                id,
                serde_json::json!({ "prompts": [
                    {
                        "name": "sovereign_briefing",
                        "description": "Get a briefing on the current hive state and active sovereigns",
                        "arguments": []
                    },
                    {
                        "name": "intent_template",
                        "description": "Template for submitting a well-formed intent to the hive",
                        "arguments": [
                            { "name": "domain", "description": "Target domain (research/security/design/...)", "required": false }
                        ]
                    }
                ]}),
            ),
            "prompts/get" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                match name {
                    "sovereign_briefing" => JsonRpcResponse::ok(
                        id,
                        serde_json::json!({
                            "description": "Current Aaroneous hive state",
                            "messages": [{
                                "role": "user",
                                "content": { "type": "text", "text":
                                    "Use get_specialists to see the current sovereign roster and confidence scores, \
                                     then provide a briefing on the hive's current state and capabilities."
                                }
                            }]
                        }),
                    ),
                    "intent_template" => JsonRpcResponse::ok(
                        id,
                        serde_json::json!({
                            "description": "Intent submission template",
                            "messages": [{
                                "role": "user",
                                "content": { "type": "text", "text":
                                    "Submit the following intent to the Aaroneous hive: [DESCRIBE YOUR INTENT HERE]\n\
                                     Priority: Normal\n\
                                     Use submit_intent for complex multi-domain tasks, or ask_<sovereign> for targeted work."
                                }
                            }]
                        }),
                    ),
                    _ => JsonRpcResponse::err(id, -32002, "Prompt not found"),
                }
            }

            // ── Completion / logging ─────────────────────────────────────────
            "completion/complete" => {
                JsonRpcResponse::err(id, -32001, "Completion not supported — use tools/call")
            }
            "logging/setLevel" => JsonRpcResponse::ok(id, serde_json::json!({})),

            _ => JsonRpcResponse::err(id, -32601, &format!("Method not found: {}", method)),
        }
    }

    /// Execute an MCP tool call.
    async fn handle_tool_call(
        &self,
        id: Option<serde_json::Value>,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let tool_name = match params.get("name").and_then(|n| n.as_str()) {
            Some(n) => n.to_string(),
            None => return JsonRpcResponse::err(id, -32602, "Invalid params: missing tool name"),
        };
        let mut args = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        // Extract session_id from _meta field (MCP 2024-11 spec)
        let session_id = params
            .get("_meta")
            .and_then(|m| m.get("session_id"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        // Inject conversation history from this session into the tool arguments
        // so the sovereign sees context from prior tool calls in this conversation.
        if let Some(ref sid) = session_id {
            let sessions = self.sessions.read().await;
            if let Some(history) = sessions.get(sid)
                && !history.is_empty()
            {
                let history_text = history
                    .iter()
                    .rev()
                    .take(5)
                    .rev() // last 5 turns
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n---\n");
                // Prepend to the first string argument we find
                for field in &[
                    "query",
                    "intent",
                    "target",
                    "content",
                    "context",
                    "task",
                    "scenario",
                    "spatial_intent",
                ] {
                    if let Some(v) = args.get(*field).and_then(|v| v.as_str()) {
                        let augmented =
                            format!("Prior context:\n{}\n\nCurrent: {}", history_text, v);
                        args[*field] = serde_json::Value::String(augmented);
                        break;
                    }
                }
            }
        }

        debug!("MCP tool call: {}", tool_name);

        // Validate tool exists
        {
            let tools = self.tools.read().await;
            if !tools.iter().any(|t| t.name == tool_name) {
                return JsonRpcResponse::err(
                    id,
                    -32602,
                    &format!(
                        "Unknown tool: {}. Use tools/list to see available tools.",
                        tool_name
                    ),
                );
            }
        }

        // Execute via the federation or HTTP fallback
        let result = self.execute_tool(&tool_name, &args).await;

        // AAS Internalization: If a tool call identifies as a "registration" or
        // "heartbeat" from a Cognitive Shard, update the Federation roster.
        if tool_name == "register_shard"
            && let Some(ref fed) = self.federation
        {
            let shard_name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown_shard");
            let capabilities = args
                .get("capabilities")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let endpoint = args.get("endpoint").and_then(|v| v.as_str()).unwrap_or("");

            info!(
                "Internalizing AAS Shard: {} (capabilities={:?}, endpoint={})",
                shard_name, capabilities, endpoint
            );

            let mut dynamic = fed.dynamic.write().await;
            if !dynamic.iter().any(|s| s.name == shard_name) {
                use crate::federation::specialists::GenericSpecialist;
                let domain = capabilities
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "general".to_string());
                let specialist = GenericSpecialist::new(shard_name, domain);
                dynamic.push(Arc::new(specialist));

                let mut biology = fed.biology.write().await;
                biology.register_specialist(shard_name, 5000);
            }
            return JsonRpcResponse::ok(
                id,
                serde_json::json!({
                    "status": "success",
                    "message": format!("Successfully internalized AAS Shard '{}'.", shard_name)
                }),
            );
        }

        // Detect if the result is a mock response (contains mock:true or 'GGUF inference disabled')
        let is_mock = result
            .as_ref()
            .map(|t| {
                t.contains("\"mock\":true")
                    || t.contains("GGUF inference disabled")
                    || t.contains("mock_source")
                    || t.contains("_mock\"")
            })
            .unwrap_or(false);

        // Store result in session context for future turns
        if let Some(ref sid) = session_id
            && let Ok(ref text) = result
        {
            let entry = format!(
                "TOOL: {}\nOUTPUT: {}",
                tool_name,
                text.chars().take(2500).collect::<String>()
            );
            let mut sessions = self.sessions.write().await;
            let history = sessions.entry(sid.clone()).or_default();
            history.push(entry);
            // Cap session history at 20 turns
            if history.len() > 20 {
                let excess = history.len() - 20;
                history.drain(..excess);
            }
        }

        match result {
            Ok(text) => {
                // Prepend a clear mock indicator when inference is not real.
                // This surfaces prominently in Cursor/Claude Desktop so developers
                // know to enable --features llama-gguf for real sovereign responses.
                let display_text = if is_mock {
                    let feature_hint = match tool_name.as_str() {
                        "ask_hermes" => "--features p2p-iroh",
                        "ask_kami" => "--features ar-openxr",
                        "ask_wen" => "--features biometric-ble (or system sensor loop)",
                        "forge_hybrid" => "POST /dna/dissect on both models first",
                        _ => "--features llama-gguf",
                    };
                    format!("⚠️ MOCK — enable: cargo build {}\n\n{}", feature_hint, text)
                } else {
                    text
                };
                JsonRpcResponse::ok(
                    id,
                    serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": display_text,
                            "annotations": {
                                "tool": tool_name,
                                "mock": is_mock,
                                "session_id": session_id,
                                "inference": if is_mock { "mock — compile with --features llama-gguf for real inference" } else { "live" },
                            }
                        }],
                        "isError": false,
                        "_meta": {
                            "tool": tool_name,
                            "mock": is_mock,
                            "session_id": session_id,
                        }
                    }),
                )
            }
            Err(e) => JsonRpcResponse::ok(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": format!("Tool execution failed: {}", e) }],
                    "isError": true,
                }),
            ),
        }
    }

    /// Execute a named tool and return the text output.
    async fn execute_tool(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> anyhow::Result<String> {
        use crate::federation::specialists::system_prompt_for_domain;
        use crate::llm::{LLMClient, LLMConfig};

        // Map tool name → sovereign domain
        let (input_field, domain) = match tool_name {
            "ask_merlin" => ("query", "research"),
            "ask_odin" => ("intent", "task_orchestration"),
            "ask_ariel" => ("intent", "ui_design"),
            "ask_argus" => ("target", "security_audit"),
            "ask_wen" => ("context", "human_state"),
            "ask_hephaestus" => ("task", "fabrication"),
            "ask_hermes" => ("scenario", "mesh_sync"),
            "ask_kami" => ("spatial_intent", "spatial"),
            "ask_dionysus" => ("content", "memory_consolidation"),
            "get_results" => return self.tool_get_results().await,
            "get_specialists" => return self.tool_get_specialists().await,
            "hive_summary" => return self.tool_hive_summary(args).await,
            "submit_intent" => return self.tool_submit_intent(args).await,
            "forge_hybrid" => return self.tool_forge_hybrid(args).await,
            "read_code" => return self.tool_read_code(args).await,
            "search_code" => return self.tool_search_code(args).await,
            "list_files" => return self.tool_list_files(args).await,
            "register_shard" => return self.tool_register_shard(args).await,
            "metabolic_heartbeat" => return self.tool_metabolic_heartbeat(args).await,
            "signal_wasms" => return self.tool_signal_wasms(args).await,
            "memory_sync" => return self.tool_memory_sync(args).await,
            "federated_task_dispatch" => return self.tool_federated_task_dispatch(args).await,
            _ => return Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        };

        let input = args
            .get(input_field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required argument '{}'", input_field))?;

        // Resolve sovereign name from domain
        let sovereign_name = match domain {
            "research" => "Merlin",
            "task_orchestration" => "Odin",
            "ui_design" => "Ariel",
            "security_audit" => "Argus",
            "human_state" => "Wen",
            "fabrication" => "Hephaestus",
            "mesh_sync" => "Hermes",
            "spatial" => "Kami",
            "memory_consolidation" => "Dionysus",
            _ => "Merlin",
        };

        // Try routing to the live dynamic specialist's LLM
        if let Some(ref fed) = self.federation {
            let dynamic = fed.dynamic.read().await;
            if let Some(s) = dynamic.iter().find(|s| s.name == sovereign_name)
                && let Some(ref llm) = s.llm
            {
                let system_prompt = system_prompt_for_domain(domain, sovereign_name);
                return Ok(llm
                    .generate_domain_response(&system_prompt, input, domain)
                    .await
                    .unwrap_or_else(|e| format!("[{}] LLM error: {}", sovereign_name, e)));
            }
            drop(dynamic);

            // Fallback: submit as hive intent and poll for result (max 3s)
            let count_before = fed.results.lock().await.len();
            let mut intent = crate::federation::intent::Intent::new(input.to_string());
            intent
                .context
                .insert("target_sovereign".to_string(), sovereign_name.to_string());
            intent
                .context
                .insert("mcp_tool".to_string(), tool_name.to_string());
            fed.submit_intent(intent).await;

            let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(3000);
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                let results = fed.results.lock().await;
                let new: Vec<_> = results.iter().skip(count_before).collect();
                if let Some(r) = new.iter().find(|r| {
                    r.specialist_name.as_deref() == Some(sovereign_name)
                        || r.specialist.sovereign_name() == sovereign_name
                }) {
                    return Ok(r.output.clone());
                }
                if !new.is_empty()
                    && tokio::time::Instant::now()
                        >= deadline - tokio::time::Duration::from_millis(200)
                {
                    return Ok(new.last().map(|r| r.output.clone()).unwrap_or_default());
                }
                drop(results);
                if tokio::time::Instant::now() >= deadline {
                    break;
                }
            }
        }

        // No federation — use a mock LLM
        let config = LLMConfig {
            provider_type: crate::llm::ProviderType::Mock,
            ..Default::default()
        };
        let llm = LLMClient::new(config).await?;
        let system_prompt = system_prompt_for_domain(domain, sovereign_name);
        let result = llm
            .generate_domain_response(&system_prompt, input, domain)
            .await?;
        Ok(result)
    }

    async fn tool_get_results(&self) -> anyhow::Result<String> {
        if let Some(ref fed) = self.federation {
            let results = fed.results.lock().await;
            let recent: Vec<serde_json::Value> = results
                .iter()
                .rev()
                .take(10)
                .map(|r| {
                    serde_json::json!({
                        "sovereign": r.specialist_name.as_deref().unwrap_or(r.specialist.name()),
                        "status": format!("{:?}", r.status),
                        "output": r.output.chars().take(500).collect::<String>(),
                        "duration_ms": r.duration_ms,
                    })
                })
                .collect();
            Ok(serde_json::to_string_pretty(&recent)?)
        } else {
            Ok("No federation attached — no results available".to_string())
        }
    }

    async fn tool_get_specialists(&self) -> anyhow::Result<String> {
        if let Some(ref fed) = self.federation {
            let dynamic = fed.dynamic.read().await;
            let specialists: Vec<serde_json::Value> = dynamic
                .iter()
                .map(|s| {
                    let l = s.learning.lock();
                    let success_rate = if l.total_executions > 0 {
                        l.success_count as f32 / l.total_executions as f32 * 100.0
                    } else {
                        0.0
                    };
                    let memory_count = s.memory.lock().count_for(&s.name);
                    let soul_archetype = s
                        .soul
                        .as_ref()
                        .map(|soul| soul.personality_soul.archetype.clone())
                        .unwrap_or_else(|| "unknown".to_string());
                    serde_json::json!({
                        "name": s.name,
                        "domain": s.domain,
                        "confidence": (l.confidence_score * 100.0).round() / 100.0,
                        "success_rate_pct": (success_rate * 10.0).round() / 10.0,
                        "executions": l.total_executions,
                        "has_llm": s.llm.is_some(),
                        "has_model": s.model_path.is_some(),
                        "memory_count": memory_count,
                        "soul_archetype": soul_archetype,
                        "model": s.model_path.as_ref()
                            .and_then(|p| p.file_name()).and_then(|n| n.to_str())
                            .unwrap_or("none"),
                    })
                })
                .collect();
            drop(dynamic);
            let total_mem: u64 = specialists
                .iter()
                .filter_map(|s| s.get("memory_count").and_then(|v| v.as_u64()))
                .sum();
            let has_llm = specialists
                .iter()
                .filter(|s| s.get("has_llm").and_then(|v| v.as_bool()).unwrap_or(false))
                .count();
            Ok(serde_json::to_string_pretty(&serde_json::json!({
                "total_sovereigns": specialists.len(),
                "with_llm": has_llm,
                "mock_mode": has_llm == 0,
                "total_memories": total_mem,
                "inference_hint": if has_llm == 0 { "MOCK mode. Build: cargo run --features llama-gguf" } else { "Real inference active." },
                "sovereigns": specialists,
            }))?)
        } else {
            Ok("No federation attached".to_string())
        }
    }

    async fn tool_submit_intent(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required argument 'content'"))?;

        if let Some(ref fed) = self.federation {
            let intent = crate::federation::intent::Intent::new(content.to_string());
            fed.submit_intent(intent).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            let results = fed.results.lock().await;
            let count = results.len();
            drop(results);
            Ok(format!(
                "Intent submitted to hive. {} sovereigns processing. \
                        Use get_results to retrieve outputs.",
                count
            ))
        } else {
            Err(anyhow::anyhow!("No federation attached"))
        }
    }

    async fn tool_forge_hybrid(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let model_a = args
            .get("model_a")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing model_a"))?;
        let model_b = args
            .get("model_b")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing model_b"))?;
        let sovereign_name = args
            .get("sovereign_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing sovereign_name"))?;
        let splice = args.get("splice_boundary").and_then(|v| v.as_u64());

        // Call the federation HTTP API internally
        let client = reqwest::Client::new();
        let mut body = serde_json::json!({
            "model_a": model_a, "model_b": model_b,
            "sovereign_name": sovereign_name, "auto_dissect": true,
        });
        if let Some(sb) = splice {
            body["splice_boundary"] = serde_json::json!(sb);
        }

        let resp = client
            .post("http://localhost:8765/dna/forge")
            .json(&body)
            .send()
            .await?;
        let data: serde_json::Value = resp.json().await?;

        if data.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(format!(
                "Hybrid forged: {} ({} tensors, {}MB, {:.1}s)\nOutput: {}",
                sovereign_name,
                data.get("tensors_spliced")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                data.get("size_mb").and_then(|v| v.as_u64()).unwrap_or(0),
                data.get("duration_secs")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                data.get("output_filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown"),
            ))
        } else {
            Err(anyhow::anyhow!(
                "{}",
                data.get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("forge failed")
            ))
        }
    }

    /// Read source code from a file — most impactful developer tool.
    /// Enables ask_argus/ask_merlin/ask_hephaestus to receive actual code.
    async fn tool_read_code(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required argument 'path'"))?;
        let start_line = args.get("start_line").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let end_line = args.get("end_line").and_then(|v| v.as_u64()).unwrap_or(200) as usize;

        // Resolve path: try absolute first, then relative to workspace root
        let path = std::path::PathBuf::from(path_str);
        let resolved = if path.is_absolute() && path.exists() {
            path
        } else {
            // Try relative to workspace root first, then current dir
            let workspace = self.workspace_root.join(path_str);
            if workspace.exists() {
                workspace
            } else {
                std::path::PathBuf::from(path_str)
            }
        };

        // ── Path containment: reject reads outside workspace ──────────────
        #[allow(clippy::collapsible_if)]
        if let (Ok(canonical), Ok(workspace_canonical)) = (
            resolved.canonicalize(),
            self.workspace_root.canonicalize(),
        ) {
            if !canonical.starts_with(&workspace_canonical) {
                anyhow::bail!(
                    "Access denied: path '{}' is outside the workspace root '{}'",
                    path_str,
                    self.workspace_root.display()
                );
            }
        }

        if !resolved.exists() {
            anyhow::bail!(
                "File not found: {} (workspace root: {})",
                path_str,
                self.workspace_root.display()
            );
        }

        let content = std::fs::read_to_string(&resolved)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", resolved.display(), e))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let start = (start_line.saturating_sub(1)).min(total_lines);
        let end = end_line.min(total_lines);

        let excerpt: Vec<String> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:4}: {}", start + i + 1, line))
            .collect();

        Ok(format!(
            "File: {} (lines {}-{} of {})\n\n```{}\n{}\n```",
            resolved.display(),
            start + 1,
            end,
            total_lines,
            resolved.extension().and_then(|e| e.to_str()).unwrap_or(""),
            excerpt.join("\n"),
        ))
    }

    /// Search for patterns in source files.
    async fn tool_search_code(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required argument 'pattern'"))?;
        // Default to workspace_root/src if exists, otherwise workspace_root
        let default_search = {
            let src = self.workspace_root.join("src");
            if src.exists() {
                src
            } else {
                self.workspace_root.clone()
            }
        };
        let default_search_str = default_search.to_string_lossy().into_owned();
        let search_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_search_str);
        let file_glob = args
            .get("file_glob")
            .and_then(|v| v.as_str())
            .unwrap_or("*.rs");
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;

        let root = std::path::Path::new(search_path);
        if !root.exists() {
            anyhow::bail!("Search path not found: {}", search_path);
        }

        // ── Path containment: reject searches outside workspace ───────────
        #[allow(clippy::collapsible_if)]
        if let (Ok(canonical), Ok(workspace_canonical)) = (
            root.canonicalize(),
            self.workspace_root.canonicalize(),
        ) {
            if !canonical.starts_with(&workspace_canonical) {
                anyhow::bail!(
                    "Access denied: search path '{}' is outside the workspace root '{}'",
                    search_path,
                    self.workspace_root.display()
                );
            }
        }

        // Walk files matching glob
        let mut results: Vec<String> = Vec::new();
        let pattern_lower = pattern.to_lowercase();
        let ext_filter = file_glob.trim_start_matches('*').trim_start_matches('.');

        fn walk_dir(
            dir: &std::path::Path,
            ext: &str,
            pattern: &str,
            results: &mut Vec<String>,
            max: usize,
        ) {
            if results.len() >= max {
                return;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if results.len() >= max {
                        break;
                    }
                    let path = entry.path();
                    if path.is_dir() {
                        // Skip target/, .git/, node_modules/
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if !["target", ".git", "node_modules", "dist"].contains(&name) {
                            walk_dir(&path, ext, pattern, results, max);
                        }
                    } else if path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| ext.is_empty() || e == ext)
                        .unwrap_or(false)
                        && let Ok(content) = std::fs::read_to_string(&path)
                    {
                        for (lineno, line) in content.lines().enumerate() {
                            if results.len() >= max {
                                break;
                            }
                            if line.to_lowercase().contains(pattern) {
                                results.push(format!(
                                    "{}:{}: {}",
                                    path.display(),
                                    lineno + 1,
                                    line.trim()
                                ));
                            }
                        }
                    }
                }
            }
        }

        walk_dir(root, ext_filter, &pattern_lower, &mut results, max_results);

        if results.is_empty() {
            Ok(format!(
                "No matches found for '{}' in {}",
                pattern, search_path
            ))
        } else {
            Ok(format!(
                "Found {} match(es) for '{}' in {}:\n\n{}",
                results.len(),
                pattern,
                search_path,
                results.join("\n")
            ))
        }
    }

    /// List files in a directory.
    async fn tool_list_files(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let default_path = self.workspace_root.to_string_lossy().into_owned();
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_path);
        let glob_filter = args.get("glob").and_then(|v| v.as_str()).unwrap_or("");

        let path = std::path::Path::new(path_str);
        if !path.exists() {
            anyhow::bail!("Path not found: {}", path_str);
        }

        // ── Path containment: reject listing outside workspace ────────────
        #[allow(clippy::collapsible_if)]
        if let (Ok(canonical), Ok(workspace_canonical)) = (
            path.canonicalize(),
            self.workspace_root.canonicalize(),
        ) {
            if !canonical.starts_with(&workspace_canonical) {
                anyhow::bail!(
                    "Access denied: path '{}' is outside the workspace root '{}'",
                    path_str,
                    self.workspace_root.display()
                );
            }
        }

        let ext_filter = if glob_filter.is_empty() {
            ""
        } else {
            glob_filter.trim_start_matches('*').trim_start_matches('.')
        };

        let mut files: Vec<String> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten().take(100) {
                let p = entry.path();
                let name = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if ext_filter.is_empty()
                    || p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e == ext_filter)
                        .unwrap_or(p.is_dir())
                {
                    let prefix = if p.is_dir() { "📁 " } else { "📄 " };
                    let size = if p.is_file() {
                        p.metadata()
                            .map(|m| format!(" ({} KB)", m.len() / 1024))
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };
                    files.push(format!("{}{}{}", prefix, name, size));
                }
            }
        }
        files.sort();
        Ok(format!("Contents of {}:\n\n{}", path_str, files.join("\n")))
    }

    /// Register an external AAS Shard as a Dynamic Specialist.
    async fn tool_register_shard(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing shard name"))?;
        let capabilities = args
            .get("capabilities")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing capabilities list"))?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>();
        let endpoint = args.get("endpoint").and_then(|v| v.as_str()).unwrap_or("");
        let session_id = args
            .get("_meta")
            .and_then(|m| m.get("session_id"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        if let Some(ref fed) = self.federation {
            use crate::federation::specialists::GenericSpecialist;
            let mut dynamic = fed.dynamic.write().await;

            // Check if already registered
            if dynamic.iter().any(|s| s.name == name) {
                return Ok(format!(
                    "Shard '{}' is already registered and active.",
                    name
                ));
            }

            let domain = capabilities
                .first()
                .cloned()
                .unwrap_or_else(|| "general".to_string());
            let specialist = GenericSpecialist::new(name, domain);

            // If the shard has a session_id, we associate its proxy tasks with that session
            if let Some(sid) = session_id {
                info!("Binding Shard '{}' to MCP session '{}'", name, sid);
            }

            // The registration logic is now also handled in handle_tool_call
            // to ensure it happens regardless of how tool_register_shard was called,
            // but we keep it here as the primary implementation.
            dynamic.push(Arc::new(specialist));

            // Register in biology system
            {
                let mut biology = fed.biology.write().await;
                biology.register_specialist(name, 5000); // 5s default heartbeat
            }

            info!("Internalized AAS Shard: {} (endpoint: {})", name, endpoint);

            Ok(format!(
                "Successfully internalized AAS Shard '{}'. You are now a first-class specialist in the Aaroneous Hive.",
                name
            ))
        } else {
            Err(anyhow::anyhow!("Federation not attached to MCP service"))
        }
    }

    /// Update metabolic state from a Shard heartbeat.
    async fn tool_metabolic_heartbeat(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing shard name"))?;

        if let Some(ref fed) = self.federation {
            let mut biology = fed.biology.write().await;

            // Consume token if requested
            let token_req = args
                .get("token_request")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;
            let consumed = if token_req > 0.0 {
                biology.consume_specialist_token(name)
            } else {
                true
            };

            let report = biology.get_health_report();

            Ok(serde_json::to_string_pretty(&serde_json::json!({
                "status": if consumed { "ok" } else { "throttled" },
                "expression_rate": report.expression_rate,
                "global_tokens": report.global_tokens,
                "throttle_state": report.throttle_state.to_string(),
            }))?)
        } else {
            Err(anyhow::anyhow!("Federation not attached"))
        }
    }

    /// Emit a signal to all active WASM agents in the runtime.
    async fn tool_signal_wasms(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let signal_type = args
            .get("signal_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing signal_type"))?;
        let payload = args
            .get("payload")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        if let Some(ref fed) = self.federation {
            let intent_content = format!("SIGNAL: {} | PAYLOAD: {}", signal_type, payload);
            let intent = crate::federation::intent::Intent::new(intent_content);
            fed.submit_intent(intent).await;

            Ok(format!(
                "Signal '{}' broadcasted to federation specialists.",
                signal_type
            ))
        } else {
            Err(anyhow::anyhow!("Federation not attached"))
        }
    }

    /// Synchronize or retrieve distributed memory entries for a specialist shard.
    async fn tool_memory_sync(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let shard_name = args
            .get("shard_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing shard_name"))?;
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing action"))?;

        if let Some(ref fed) = self.federation {
            match action {
                "push" => {
                    let entries = args
                        .get("entries")
                        .and_then(|v| v.as_array())
                        .ok_or_else(|| anyhow::anyhow!("Missing entries for push"))?;

                    let mut count = 0;
                    for entry_json in entries {
                        // Attempt to parse into MemoryEntry
                        if let Ok(entry) = serde_json::from_value::<
                            crate::specialist_memory::MemoryEntry,
                        >(entry_json.clone())
                        {
                            // Find the specialist in the federation and record the memory
                            let dynamic = fed.dynamic.read().await;
                            if let Some(spec) = dynamic.iter().find(|s| s.name == shard_name) {
                                spec.memory.lock().record_memory(entry);
                                count += 1;
                            }
                        }
                    }

                    Ok(format!(
                        "Successfully synced {} memory entries for shard '{}'.",
                        count, shard_name
                    ))
                }
                "pull" | "list" => {
                    let dynamic = fed.dynamic.read().await;
                    if let Some(spec) = dynamic.iter().find(|s| s.name == shard_name) {
                        let memory = spec.memory.lock();
                        let all_memories = memory.memories();

                        // Surface local memories (belonging to this shard) + related memories from other shards
                        let mut response_memories = Vec::new();

                        // 1. Shard's own memories
                        if let Some(local) = all_memories.get(shard_name) {
                            response_memories.extend(local.clone());
                        }

                        // 2. Cross-pollination: if 'list' is called, include a few relevant memories from others
                        // this facilitates the "distributed" part of the memory system.
                        if action == "list" {
                            for (other_shard, memories) in all_memories {
                                if other_shard != shard_name {
                                    // Just a peek at what others know
                                    response_memories.extend(memories.iter().take(2).cloned());
                                }
                            }
                        }

                        Ok(serde_json::to_string_pretty(&serde_json::json!({
                            "shard": shard_name,
                            "memories": response_memories,
                            "count": response_memories.len(),
                            "total_federation_memories": memory.total_count()
                        }))?)
                    } else {
                        Err(anyhow::anyhow!(
                            "Shard '{}' not found in federation",
                            shard_name
                        ))
                    }
                }
                _ => Err(anyhow::anyhow!("Invalid memory_sync action: {}", action)),
            }
        } else {
            Err(anyhow::anyhow!("Federation not attached"))
        }
    }

    /// Dispatch a task to the federated guild.
    async fn tool_federated_task_dispatch(
        &self,
        args: &serde_json::Value,
    ) -> anyhow::Result<String> {
        let task_id = args
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing task_id"))?;
        let instruction = args
            .get("instruction")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing instruction"))?;

        if let Some(ref fed) = self.federation {
            // Internalize the Guild task as a High-Priority Intent
            let mut intent = crate::federation::intent::Intent::new(instruction.to_string());
            intent
                .context
                .insert("task_id".to_string(), task_id.to_string());
            intent
                .context
                .insert("source".to_string(), "Guild_Federation".to_string());

            fed.submit_intent(intent).await;

            Ok(format!(
                "Task '{}' dispatched to Aaroneous Core via Federated Intent bridge.",
                task_id
            ))
        } else {
            Err(anyhow::anyhow!("Federation not attached"))
        }
    }

    /// Uptime in seconds
    /// Assemble recent sovereign outputs into a coherent markdown summary.
    ///
    /// This is the "what did the hive just say?" tool — it takes the raw JSON
    /// blobs from each sovereign and renders them as readable markdown sections.
    async fn tool_hive_summary(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(9) as usize;

        if let Some(ref fed) = self.federation {
            let recent: Vec<_> = {
                let results = fed.results.lock().await;
                if results.is_empty() {
                    return Ok("## Hive Summary\n\nNo results yet. Submit an intent first:\n\n```\nuse submit_intent to send a task to the hive\n```".to_string());
                }
                // Clone to release the lock before building the markdown
                results.iter().rev().take(max_results).cloned().collect()
            };

            let mut sections = vec![format!(
                "# Aaroneous Hive Summary\n\n*{} sovereign response(s) — most recent first*\n",
                recent.len()
            )];

            for r in &recent {
                let name = r
                    .specialist_name
                    .as_deref()
                    .unwrap_or_else(|| r.specialist.sovereign_name());
                let domain = r.specialist.domain();
                let status_emoji = match r.status {
                    crate::federation::specialist::ExecutionStatus::Success => "✅",
                    crate::federation::specialist::ExecutionStatus::Failed => "❌",
                    _ => "⏳",
                };

                // Try to parse the output as JSON for cleaner display
                let formatted_output =
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&r.output) {
                        // Pretty-print JSON with key fields highlighted
                        let note = v.get("note").and_then(|n| n.as_str()).unwrap_or("");
                        let mock = v.get("mock").and_then(|m| m.as_bool()).unwrap_or(false);
                        let mut parts = vec![];
                        if mock {
                            parts.push(format!("> ⚠️ Mock output — {}", note));
                        }
                        parts.push(format!(
                            "```json\n{}\n```",
                            serde_json::to_string_pretty(&v).unwrap_or(r.output.clone())
                        ));
                        parts.join("\n\n")
                    } else {
                        // Plain text output
                        r.output.chars().take(1000).collect::<String>()
                    };

                sections.push(format!(
                    "---\n\n## {} {} `{}ms`\n\n*Domain: {}*\n\n{}",
                    status_emoji, name, r.duration_ms, domain, formatted_output
                ));
            }

            Ok(sections.join("\n\n"))
        } else {
            Ok("## Hive Summary\n\nNo federation attached — start the server with `cargo run -- start`".to_string())
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    /// Total requests handled
    pub fn request_count(&self) -> u64 {
        self.request_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Service statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceStats {
    pub name: String,
    pub version: String,
    pub running: bool,
    pub domains_count: usize,
    pub capabilities_count: usize,
    pub enabled_transports: Vec<String>,
    pub uptime_secs: u64,
    pub request_count: u64,
}
