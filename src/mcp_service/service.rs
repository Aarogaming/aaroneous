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
use std::collections::HashMap;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};

use crate::mcp_service::{ServiceConfig, Capability, CapabilityDomain};
use crate::federation::hive::Federation;

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
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }
    pub fn err(id: Option<serde_json::Value>, code: i32, message: &str) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: None,
               error: Some(JsonRpcError { code, message: message.into(), data: None }) }
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
    pub fn new(name: &str, description: &str, props: serde_json::Value, required: Vec<&str>) -> Self {
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
    /// Key: session_id (from MCP _meta.session_id field in request)
    /// Value: Vec of prior tool outputs formatted as "TOOL: <name>\nOUTPUT: <text>"
    pub sessions: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl McpService {
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            config,
            federation: None,
            tools: Arc::new(RwLock::new(Vec::new())),
            domains: Arc::new(RwLock::new(HashMap::new())),
            started_at: std::time::Instant::now(),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Attach the live federation so tools can call sovereigns.
    pub fn with_federation(mut self, federation: Arc<Federation>) -> Self {
        self.federation = Some(federation);
        self
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
        tools.push(McpTool::new("get_results",
            "Retrieve the most recent execution results from all sovereigns. \
             Returns the last 10 specialist outputs.",
            serde_json::json!({}),
            vec![],
        ));
        tools.push(McpTool::new("get_specialists",
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
        let params = raw.get("params").cloned().unwrap_or(serde_json::Value::Null);

        self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        debug!("MCP request: method={}", method);

        match method.as_str() {
            // ── MCP lifecycle ──────────────────────────────────────────────
            "initialize" => {
                JsonRpcResponse::ok(id, serde_json::json!({
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
                }))
            }
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
            "tools/call" => {
                self.handle_tool_call(id, params).await
            }

            // ── Resources (empty — no file resources exposed) ──────────────
            "resources/list" => {
                JsonRpcResponse::ok(id, serde_json::json!({ "resources": [] }))
            }
            "resources/read" => {
                JsonRpcResponse::err(id, -32002, "No resources available")
            }

            // ── Prompts ─────────────────────────────────────────────────────
            "prompts/list" => {
                JsonRpcResponse::ok(id, serde_json::json!({ "prompts": [
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
                ]}))
            }
            "prompts/get" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                match name {
                    "sovereign_briefing" => JsonRpcResponse::ok(id, serde_json::json!({
                        "description": "Current Aaroneous hive state",
                        "messages": [{
                            "role": "user",
                            "content": { "type": "text", "text":
                                "Use get_specialists to see the current sovereign roster and confidence scores, \
                                 then provide a briefing on the hive's current state and capabilities."
                            }
                        }]
                    })),
                    "intent_template" => JsonRpcResponse::ok(id, serde_json::json!({
                        "description": "Intent submission template",
                        "messages": [{
                            "role": "user",
                            "content": { "type": "text", "text":
                                "Submit the following intent to the Aaroneous hive: [DESCRIBE YOUR INTENT HERE]\n\
                                 Priority: Normal\n\
                                 Use submit_intent for complex multi-domain tasks, or ask_<sovereign> for targeted work."
                            }
                        }]
                    })),
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
        let mut args = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));

        // Extract session_id from _meta field (MCP 2024-11 spec)
        let session_id = params.get("_meta")
            .and_then(|m| m.get("session_id"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        // Inject conversation history from this session into the tool arguments
        // so the sovereign sees context from prior tool calls in this conversation.
        if let Some(ref sid) = session_id {
            let sessions = self.sessions.read().await;
            if let Some(history) = sessions.get(sid) {
                if !history.is_empty() {
                    let history_text = history.iter()
                        .rev().take(5).rev() // last 5 turns
                        .cloned().collect::<Vec<_>>().join("\n---\n");
                    // Prepend to the first string argument we find
                    for field in &["query", "intent", "target", "content", "context", "task",
                                   "scenario", "spatial_intent"] {
                        if let Some(v) = args.get(*field).and_then(|v| v.as_str()) {
                            let augmented = format!("Prior context:\n{}\n\nCurrent: {}", history_text, v);
                            args[*field] = serde_json::Value::String(augmented);
                            break;
                        }
                    }
                }
            }
        }

        debug!("MCP tool call: {}", tool_name);

        // Validate tool exists
        {
            let tools = self.tools.read().await;
            if !tools.iter().any(|t| t.name == tool_name) {
                return JsonRpcResponse::err(id, -32602,
                    &format!("Unknown tool: {}. Use tools/list to see available tools.", tool_name));
            }
        }

        // Execute via the federation or HTTP fallback
        let result = self.execute_tool(&tool_name, &args).await;

        // Detect if the result is a mock response (contains mock:true or 'GGUF inference disabled')
        let is_mock = result.as_ref().map(|t| {
            t.contains("\"mock\":true") || t.contains("GGUF inference disabled")
                || t.contains("mock_source") || t.contains("_mock\"")
        }).unwrap_or(false);

        // Store result in session context for future turns
        if let Some(ref sid) = session_id {
            if let Ok(ref text) = result {
                let entry = format!("TOOL: {}\nOUTPUT: {}",
                    tool_name, text.chars().take(800).collect::<String>());
                let mut sessions = self.sessions.write().await;
                let history = sessions.entry(sid.clone()).or_default();
                history.push(entry);
                // Cap session history at 20 turns
                if history.len() > 20 {
                    let excess = history.len() - 20;
                    history.drain(..excess);
                }
            }
        }

        match result {
            Ok(text) => JsonRpcResponse::ok(id, serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": text,
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
            })),
            Err(e) => JsonRpcResponse::ok(id, serde_json::json!({
                "content": [{ "type": "text", "text": format!("Tool execution failed: {}", e) }],
                "isError": true,
            })),
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
            "ask_merlin"      => ("query",          "research"),
            "ask_odin"        => ("intent",         "task_orchestration"),
            "ask_ariel"       => ("intent",         "ui_design"),
            "ask_argus"       => ("target",         "security_audit"),
            "ask_wen"         => ("context",        "human_state"),
            "ask_hephaestus"  => ("task",           "fabrication"),
            "ask_hermes"      => ("scenario",       "mesh_sync"),
            "ask_kami"        => ("spatial_intent", "spatial"),
            "ask_dionysus"    => ("content",        "memory_consolidation"),
            "get_results"     => return self.tool_get_results().await,
            "get_specialists" => return self.tool_get_specialists().await,
            "submit_intent"   => return self.tool_submit_intent(args).await,
            "forge_hybrid"    => return self.tool_forge_hybrid(args).await,
            _                 => return Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        };

        let input = args.get(input_field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required argument '{}'", input_field))?;

        // Resolve sovereign name from domain
        let sovereign_name = match domain {
            "research"            => "Merlin",
            "task_orchestration"  => "Odin",
            "ui_design"           => "Ariel",
            "security_audit"      => "Argus",
            "human_state"         => "Wen",
            "fabrication"         => "Hephaestus",
            "mesh_sync"           => "Hermes",
            "spatial"             => "Kami",
            "memory_consolidation"=> "Dionysus",
            _                     => "Merlin",
        };

        // Try routing to the live dynamic specialist's LLM
        if let Some(ref fed) = self.federation {
            let dynamic = fed.dynamic.read().await;
            if let Some(s) = dynamic.iter().find(|s| s.name == sovereign_name) {
                if let Some(ref llm) = s.llm {
                    let system_prompt = system_prompt_for_domain(domain, sovereign_name);
                    return Ok(llm.generate_domain_response(&system_prompt, input, domain)
                        .await
                        .unwrap_or_else(|e| format!("[{}] LLM error: {}", sovereign_name, e)));
                }
            }
            drop(dynamic);

            // Fallback: submit as hive intent and poll for result (max 3s)
            let count_before = fed.results.lock().await.len();
            let mut intent = crate::federation::intent::Intent::new(input.to_string());
            intent.context.insert("target_sovereign".to_string(), sovereign_name.to_string());
            intent.context.insert("mcp_tool".to_string(), tool_name.to_string());
            fed.submit_intent(intent).await;

            let deadline = tokio::time::Instant::now()
                + tokio::time::Duration::from_millis(3000);
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
                if tokio::time::Instant::now() >= deadline { break; }
            }
        }

        // No federation — use a mock LLM
        let config = LLMConfig { provider_type: crate::llm::ProviderType::Mock, ..Default::default() };
        let llm = LLMClient::new(config).await?;
        let system_prompt = system_prompt_for_domain(domain, sovereign_name);
        let result = llm.generate_domain_response(&system_prompt, input, domain).await?;
        Ok(result)
    }

    async fn tool_get_results(&self) -> anyhow::Result<String> {
        if let Some(ref fed) = self.federation {
            let results = fed.results.lock().await;
            let recent: Vec<serde_json::Value> = results.iter().rev().take(10).map(|r| {
                serde_json::json!({
                    "sovereign": r.specialist_name.as_deref().unwrap_or(r.specialist.name()),
                    "status": format!("{:?}", r.status),
                    "output": r.output.chars().take(500).collect::<String>(),
                    "duration_ms": r.duration_ms,
                })
            }).collect();
            Ok(serde_json::to_string_pretty(&recent)?)
        } else {
            Ok("No federation attached — no results available".to_string())
        }
    }

    async fn tool_get_specialists(&self) -> anyhow::Result<String> {
        if let Some(ref fed) = self.federation {
            let dynamic = fed.dynamic.read().await;
            let specialists: Vec<serde_json::Value> = dynamic.iter().map(|s| {
                let l = s.learning.lock();
                serde_json::json!({
                    "name": s.name,
                    "domain": s.domain,
                    "confidence": l.confidence_score,
                    "executions": l.total_executions,
                    "has_model": s.model_path.is_some(),
                })
            }).collect();
            drop(dynamic);
            Ok(serde_json::to_string_pretty(&specialists)?)
        } else {
            Ok("No federation attached".to_string())
        }
    }

    async fn tool_submit_intent(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let content = args.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required argument 'content'"))?;

        if let Some(ref fed) = self.federation {
            let intent = crate::federation::intent::Intent::new(content.to_string());
            fed.submit_intent(intent).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            let results = fed.results.lock().await;
            let count = results.len();
            drop(results);
            Ok(format!("Intent submitted to hive. {} sovereigns processing. \
                        Use get_results to retrieve outputs.", count))
        } else {
            Err(anyhow::anyhow!("No federation attached"))
        }
    }

    async fn tool_forge_hybrid(&self, args: &serde_json::Value) -> anyhow::Result<String> {
        let model_a = args.get("model_a").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing model_a"))?;
        let model_b = args.get("model_b").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing model_b"))?;
        let sovereign_name = args.get("sovereign_name").and_then(|v| v.as_str())
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

        let resp = client.post("http://localhost:8765/dna/forge")
            .json(&body).send().await?;
        let data: serde_json::Value = resp.json().await?;

        if data.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(format!(
                "Hybrid forged: {} ({} tensors, {}MB, {:.1}s)\nOutput: {}",
                sovereign_name,
                data.get("tensors_spliced").and_then(|v| v.as_u64()).unwrap_or(0),
                data.get("size_mb").and_then(|v| v.as_u64()).unwrap_or(0),
                data.get("duration_secs").and_then(|v| v.as_f64()).unwrap_or(0.0),
                data.get("output_filename").and_then(|v| v.as_str()).unwrap_or("unknown"),
            ))
        } else {
            Err(anyhow::anyhow!("{}", data.get("error").and_then(|v| v.as_str()).unwrap_or("forge failed")))
        }
    }

    /// Uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    /// Total requests handled
    pub fn request_count(&self) -> u64 {
        self.request_count.load(std::sync::atomic::Ordering::Relaxed)
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
