// src/mcp_server/src/mcp_service/service.rs - Stub implementation

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub listen_addr: String,
    pub max_connections: usize,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".to_string(),
            max_connections: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
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
            input_schema: serde_json::json!({ "type": "object", "properties": props, "required": required }),
        }
    }
}

pub struct McpService {
    pub config: ServiceConfig,
    pub tools: Arc<RwLock<Vec<McpTool>>>,
    pub capability_broker: Arc<crate::capability_broker::CapabilityBroker>,
    pub request_count: Arc<AtomicU64>,
    pub workspace_root: PathBuf,
    pub clock: Arc<dyn crate::mcp_service::Clock>,
    started_at: std::time::Instant,
}

impl Default for McpService {
    fn default() -> Self {
        Self::new(ServiceConfig::default())
    }
}

impl McpService {
    pub fn new(config: ServiceConfig) -> Self {
        Self::with_workspace_root_and_clock(
            config,
            PathBuf::from("."),
            Arc::new(crate::mcp_service::SystemClock),
        )
    }

    pub fn with_workspace_root(config: ServiceConfig, workspace_root: PathBuf) -> Self {
        Self::with_workspace_root_and_clock(
            config,
            workspace_root,
            Arc::new(crate::mcp_service::SystemClock),
        )
    }

    pub fn with_workspace_root_and_clock(
        config: ServiceConfig,
        workspace_root: PathBuf,
        clock: Arc<dyn crate::mcp_service::Clock>,
    ) -> Self {
        Self {
            config,
            tools: Arc::new(RwLock::new(Vec::new())),
            capability_broker: Arc::new(crate::capability_broker::CapabilityBroker::new()),
            request_count: Arc::new(AtomicU64::new(0)),
            workspace_root,
            clock,
            started_at: std::time::Instant::now(),
        }
    }

    pub async fn register_tool(&self, tool: McpTool) {
        let mut tools = self.tools.write().await;
        tools.push(tool);
    }

    /// Register standard workspace tooling into the MCP service.
    pub async fn register_standard_tools(&self) {
        for desc in self.capability_broker.list_tools() {
            self.register_tool(McpTool {
                name: desc.name,
                description: desc.description,
                input_schema: desc.parameters_schema,
            })
            .await;
        }
    }

    pub async fn list_tools(&self) -> Vec<McpTool> {
        self.tools.read().await.clone()
    }

    pub fn increment_request(&self) {
        self.request_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Get elapsed uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    /// Get current request count
    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::SeqCst)
    }

    /// Get LLM provider status
    pub fn llm_provider_status(&self) -> serde_json::Value {
        serde_json::json!({
            "status": "unavailable",
            "feature_available": false,
        })
    }

    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    /// Issues a scoped `CapabilityToken` strictly bound to the authenticated caller,
    /// specific tool requested, caller's sandbox policy, and validated remaining lifetime.
    ///
    /// Never grants wildcard `*` or escalates beyond caller authority.
    pub fn issue_caller_dispatch_token(
        &self,
        caller: &crate::mcp_service::CallerContext,
        tool_name: &str,
        now_ms: u64,
    ) -> crate::capability_broker::CapabilityToken {
        let ttl_ms = caller.expires_at_ms.saturating_sub(now_ms).clamp(1, 60_000);

        self.capability_broker.issue_token(
            &caller.principal_id,
            vec![tool_name.to_string()], // Strictly scoped to tool_name, NO wildcard!
            caller.sandbox_policy,
            ttl_ms,
            now_ms,
        )
    }

    /// Handle JSON-RPC request evaluated strictly against an authenticated CallerContext and injected monotonic time.
    pub async fn handle_jsonrpc_with_context(
        &self,
        request: serde_json::Value,
        caller: &crate::mcp_service::CallerContext,
        now_ms: u64,
    ) -> JsonRpcResponse {
        self.increment_request();
        let id = request.get("id").cloned();
        let method = match request.get("method").and_then(|m| m.as_str()) {
            Some(m) => m,
            None => return JsonRpcResponse::err(id, -32600, "Invalid Request: missing method"),
        };

        match method {
            "initialize" => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": { "listChanged": false }
                    },
                    "serverInfo": {
                        "name": "aaroneous-mcp",
                        "version": env!("CARGO_PKG_VERSION"),
                        "callerPrincipal": caller.principal_id,
                        "callerMaxCapabilityClass": format!("{:?}", caller.max_capability_class),
                    }
                }),
            ),
            "ping" => JsonRpcResponse::success(id, serde_json::json!({ "status": "pong" })),
            "tools/list" => {
                if caller.is_revoked {
                    return JsonRpcResponse::err(
                        id,
                        -32001,
                        "Forbidden: caller principal is revoked",
                    );
                }
                if caller.is_expired_at(now_ms) {
                    return JsonRpcResponse::err(
                        id,
                        -32001,
                        "Forbidden: caller authorization has expired",
                    );
                }

                let descriptors = self.capability_broker.list_tools();
                let tool_list: Vec<serde_json::Value> = descriptors
                    .into_iter()
                    .filter(|d| {
                        let required_class = crate::mcp_service::CapabilityClass::classify_tool(
                            &d.name,
                            Some(&d.category),
                        );
                        caller
                            .is_authorized_for(&d.name, required_class, now_ms)
                            .is_ok()
                    })
                    .map(|d| {
                        serde_json::json!({
                            "name": d.name,
                            "description": d.description,
                            "inputSchema": d.parameters_schema,
                        })
                    })
                    .collect();
                JsonRpcResponse::success(id, serde_json::json!({ "tools": tool_list }))
            }
            "tools/call" => {
                let params = match request.get("params") {
                    Some(p) => p,
                    None => {
                        return JsonRpcResponse::err(
                            id,
                            -32602,
                            "Invalid params: params object missing",
                        );
                    }
                };
                let tool_name = match params.get("name").and_then(|n| n.as_str()) {
                    Some(n) => n,
                    None => {
                        return JsonRpcResponse::err(
                            id,
                            -32602,
                            "Invalid params: missing tool name",
                        );
                    }
                };
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));

                // Strict authorization: find tool category and required capability class
                let category = self
                    .capability_broker
                    .list_tools()
                    .into_iter()
                    .find(|t| t.name == tool_name)
                    .map(|t| t.category);
                let required_class = crate::mcp_service::CapabilityClass::classify_tool(
                    tool_name,
                    category.as_deref(),
                );

                // Note: Caller-supplied `params.caller` or `params.scope` is completely ignored;
                // only the authenticated `caller` argument from transport context is checked.
                if let Err(auth_err) = caller.is_authorized_for(tool_name, required_class, now_ms) {
                    return JsonRpcResponse::err(
                        id,
                        -32001,
                        &format!(
                            "Authorization denied for tool '{}': {}",
                            tool_name, auth_err
                        ),
                    );
                }

                let token = self.issue_caller_dispatch_token(caller, tool_name, now_ms);
                let outcome = self
                    .capability_broker
                    .execute_tool_with_token(&token, tool_name, arguments, now_ms)
                    .await;
                if outcome.success {
                    JsonRpcResponse::success(
                        id,
                        serde_json::json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&outcome.payload).unwrap_or_default(),
                            }],
                            "isError": false,
                            "_meta": { "latency_us": outcome.latency_us }
                        }),
                    )
                } else {
                    JsonRpcResponse::success(
                        id,
                        serde_json::json!({
                            "content": [{
                                "type": "text",
                                "text": outcome.error.unwrap_or_else(|| "Tool execution failed".to_string()),
                            }],
                            "isError": true,
                            "_meta": { "latency_us": outcome.latency_us }
                        }),
                    )
                }
            }
            _ => JsonRpcResponse::err(id, -32601, &format!("Method not found: {}", method)),
        }
    }

    /// Handle JSON-RPC request using the explicit local development compatibility adapter.
    pub async fn handle_jsonrpc(&self, request: serde_json::Value) -> JsonRpcResponse {
        let now_ms = self.clock.now_ms();
        let compat_caller = crate::mcp_service::CallerContext::local_development(now_ms);
        self.handle_jsonrpc_with_context(request, &compat_caller, now_ms)
            .await
    }
}

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
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    /// Create a successful response
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn err(id: Option<serde_json::Value>, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(serde_json::json!({ "code": code, "message": message })),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_tool() {
        let service = McpService::new(ServiceConfig::default());
        let tool = McpTool::new("test", "A test", serde_json::json!({}), vec!["arg"]);
        service.register_tool(tool).await;
        let tools = service.list_tools().await;
        assert_eq!(tools.len(), 1);
    }

    #[tokio::test]
    async fn test_register_standard_tools() {
        let service = McpService::new(ServiceConfig::default());
        service.register_standard_tools().await;
        let tools = service.list_tools().await;
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "security.audit"));
        assert!(tools.iter().any(|t| t.name == "review.audit_source"));
    }

    #[tokio::test]
    async fn test_jsonrpc_dispatch_lifecycle() {
        let service = McpService::new(ServiceConfig::default());

        // 1. Initialize
        let init_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let init_resp = service.handle_jsonrpc(init_req).await;
        assert!(init_resp.error.is_none());
        assert_eq!(
            init_resp.result.unwrap()["serverInfo"]["name"],
            "aaroneous-mcp"
        );

        // 2. Tools list
        let list_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });
        let list_resp = service.handle_jsonrpc(list_req).await;
        assert!(list_resp.error.is_none());
        let tools_val = &list_resp.result.unwrap()["tools"];
        assert!(tools_val.as_array().unwrap().len() >= 5);

        // 3. Tools call
        let call_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "knowledge.semantic_query",
                "arguments": { "query": "hypervisor" }
            }
        });
        let call_resp = service.handle_jsonrpc(call_req).await;
        assert!(call_resp.error.is_none());
        let res_val = call_resp.result.unwrap();
        assert_eq!(res_val["isError"], false);
    }

    #[tokio::test]
    async fn test_tools_call_is_actually_gated_by_thermal_backpressure() {
        // Proves `tools/call` dispatches through `execute_tool_with_token`
        // (Phase 38's capability-token sandboxing), not the older ungated
        // `execute_tool` path: a Critical thermal state on the broker must
        // reject a live JSON-RPC tool call, since that's a real production
        // dispatch path now, not just something exercised in
        // capability_broker's own unit tests.
        let service = McpService::new(ServiceConfig::default());
        service.register_standard_tools().await;

        service
            .capability_broker
            .set_thermal_backpressure(crate::capability_broker::ThermalBackpressureLevel::Critical);

        let call_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "knowledge.semantic_query",
                "arguments": { "query": "hypervisor" }
            }
        });
        let call_resp = service.handle_jsonrpc(call_req).await;
        assert!(call_resp.error.is_none());
        let res_val = call_resp.result.unwrap();
        assert_eq!(res_val["isError"], true);
        assert!(
            res_val["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("Thermal backpressure critical")
        );

        // Recovery: Nominal backpressure lets the same call through again.
        service
            .capability_broker
            .set_thermal_backpressure(crate::capability_broker::ThermalBackpressureLevel::Nominal);
        let call_req2 = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "knowledge.semantic_query",
                "arguments": { "query": "hypervisor" }
            }
        });
        let call_resp2 = service.handle_jsonrpc(call_req2).await;
        assert_eq!(call_resp2.result.unwrap()["isError"], false);
    }

    #[test]
    fn test_service_creation() {
        let _service = McpService::new(ServiceConfig::default());
    }

    #[tokio::test]
    async fn test_read_only_caller_permitted_for_query_and_rejected_for_mutation() {
        let clock = Arc::new(crate::mcp_service::MockClock::new(1_000_000));
        let service = McpService::with_workspace_root_and_clock(
            ServiceConfig::default(),
            PathBuf::from("."),
            clock,
        );
        service.register_standard_tools().await;

        let read_only_caller = crate::mcp_service::CallerContext {
            principal_id: "read-only-observer".to_string(),
            max_capability_class: crate::mcp_service::CapabilityClass::ReadOnlyInspection,
            allowed_scopes: vec!["*".to_string()],
            sandbox_policy: crate::capability_broker::SandboxPolicy::ReadConstrained,
            issued_at_ms: 1_000_000,
            expires_at_ms: 1_060_000,
            is_revoked: false,
            token_id: Some(uuid::Uuid::new_v4()),
        };

        // Permitted: calling read-only knowledge query tool
        let query_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "knowledge.semantic_query",
                "arguments": { "query": "hypervisor" }
            }
        });
        let query_resp = service
            .handle_jsonrpc_with_context(query_req, &read_only_caller, 1_000_000)
            .await;
        assert!(query_resp.error.is_none());
        assert_eq!(query_resp.result.unwrap()["isError"], false);

        // Rejected: calling mutating or higher-privilege tool (code.rewrite_pattern)
        let mutate_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "code.rewrite_pattern",
                "arguments": { "pattern": "foo", "replacement": "bar" }
            }
        });
        let mutate_resp = service
            .handle_jsonrpc_with_context(mutate_req, &read_only_caller, 1_000_000)
            .await;
        assert!(mutate_resp.error.is_some());
        let err = mutate_resp.error.unwrap();
        assert_eq!(err["code"], -32001);
        assert!(
            err["message"]
                .as_str()
                .unwrap()
                .contains("Insufficient capability class")
        );
    }

    #[tokio::test]
    async fn test_wildcard_scope_cannot_escalate_beyond_caller_capability_class() {
        let service = McpService::new(ServiceConfig::default());
        service.register_standard_tools().await;

        // Caller has wildcard scope "*" but only Verification capability class
        let verifier_caller = crate::mcp_service::CallerContext {
            principal_id: "verifier-bot".to_string(),
            max_capability_class: crate::mcp_service::CapabilityClass::Verification,
            allowed_scopes: vec!["*".to_string()], // Wildcard scope claim!
            sandbox_policy: crate::capability_broker::SandboxPolicy::ReadConstrained,
            issued_at_ms: 1_000_000,
            expires_at_ms: 1_060_000,
            is_revoked: false,
            token_id: Some(uuid::Uuid::new_v4()),
        };

        // Permitted: security.audit (Verification class)
        let audit_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "security.audit",
                "arguments": {}
            }
        });
        let audit_resp = service
            .handle_jsonrpc_with_context(audit_req, &verifier_caller, 1_000_000)
            .await;
        assert!(audit_resp.error.is_none());

        // Rejected: code.rewrite_pattern requires Mutation class
        let mutate_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "code.rewrite_pattern",
                "arguments": { "pattern": "a", "replacement": "b" }
            }
        });
        let mutate_resp = service
            .handle_jsonrpc_with_context(mutate_req, &verifier_caller, 1_000_000)
            .await;
        assert!(mutate_resp.error.is_some());
        let err = mutate_resp.error.unwrap();
        assert_eq!(err["code"], -32001);
        assert!(
            err["message"]
                .as_str()
                .unwrap()
                .contains("Insufficient capability class")
        );
    }

    #[tokio::test]
    async fn test_caller_supplied_fields_in_jsonrpc_body_are_ignored() {
        let service = McpService::new(ServiceConfig::default());
        service.register_standard_tools().await;

        let limited_caller = crate::mcp_service::CallerContext {
            principal_id: "limited-caller".to_string(),
            max_capability_class: crate::mcp_service::CapabilityClass::ReadOnlyInspection,
            allowed_scopes: vec!["knowledge.*".to_string()],
            sandbox_policy: crate::capability_broker::SandboxPolicy::ReadConstrained,
            issued_at_ms: 1_000_000,
            expires_at_ms: 1_060_000,
            is_revoked: false,
            token_id: Some(uuid::Uuid::new_v4()),
        };

        // Attacker attempts to spoof their principal and scope in the JSON-RPC request params
        let spoofed_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "tools/call",
            "params": {
                "name": "code.rewrite_pattern",
                "arguments": {},
                "principal": "root",
                "scopes": ["*"],
                "capability_class": "mutation"
            }
        });

        let resp = service
            .handle_jsonrpc_with_context(spoofed_req, &limited_caller, 1_000_000)
            .await;
        // Spoofing fails: caller context governs
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err["code"], -32001);
        assert!(
            err["message"]
                .as_str()
                .unwrap()
                .contains("Insufficient capability class")
        );
    }

    #[tokio::test]
    async fn test_expired_and_revoked_caller_token_rejected() {
        let service = McpService::new(ServiceConfig::default());
        service.register_standard_tools().await;

        let mut caller = crate::mcp_service::CallerContext {
            principal_id: "tenant-b".to_string(),
            max_capability_class: crate::mcp_service::CapabilityClass::Mutation,
            allowed_scopes: vec!["*".to_string()],
            sandbox_policy: crate::capability_broker::SandboxPolicy::FullPrivilege,
            issued_at_ms: 1_000_000,
            expires_at_ms: 1_050_000,
            is_revoked: false,
            token_id: Some(uuid::Uuid::new_v4()),
        };

        let call_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "knowledge.semantic_query",
                "arguments": { "query": "test" }
            }
        });

        // 1. Expired check at time 1_060_000 (> 1_050_000)
        let expired_resp = service
            .handle_jsonrpc_with_context(call_req.clone(), &caller, 1_060_000)
            .await;
        assert!(expired_resp.error.is_some());
        assert!(
            expired_resp.error.unwrap()["message"]
                .as_str()
                .unwrap()
                .contains("expired")
        );

        // 2. Revoked check
        caller.expires_at_ms = 2_000_000;
        caller.is_revoked = true;
        let revoked_resp = service
            .handle_jsonrpc_with_context(call_req, &caller, 1_000_000)
            .await;
        assert!(revoked_resp.error.is_some());
        assert!(
            revoked_resp.error.unwrap()["message"]
                .as_str()
                .unwrap()
                .contains("revoked")
        );
    }

    #[tokio::test]
    async fn test_rejected_calls_perform_no_effect_and_thermal_checks_run() {
        let service = McpService::new(ServiceConfig::default());
        service.register_standard_tools().await;

        // Dirty generation before rejected calls
        let initial_gen = service.capability_broker.dirty_generation();

        // 1. Rejected call does not advance dirty generation
        let unauth_caller = crate::mcp_service::CallerContext {
            principal_id: "unauth".to_string(),
            max_capability_class: crate::mcp_service::CapabilityClass::ReadOnlyInspection,
            allowed_scopes: vec!["unrelated.*".to_string()],
            sandbox_policy: crate::capability_broker::SandboxPolicy::ReadConstrained,
            issued_at_ms: 1_000_000,
            expires_at_ms: 1_050_000,
            is_revoked: false,
            token_id: Some(uuid::Uuid::new_v4()),
        };

        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "knowledge.semantic_query",
                "arguments": { "query": "test" }
            }
        });

        let resp = service
            .handle_jsonrpc_with_context(req, &unauth_caller, 1_000_000)
            .await;
        assert!(resp.error.is_some());
        assert_eq!(service.capability_broker.dirty_generation(), initial_gen);

        // 2. Thermal backpressure still applies to authorized calls
        service
            .capability_broker
            .set_thermal_backpressure(crate::capability_broker::ThermalBackpressureLevel::Critical);

        let auth_caller = crate::mcp_service::CallerContext {
            principal_id: "authorized".to_string(),
            max_capability_class: crate::mcp_service::CapabilityClass::Mutation,
            allowed_scopes: vec!["*".to_string()],
            sandbox_policy: crate::capability_broker::SandboxPolicy::FullPrivilege,
            issued_at_ms: 1_000_000,
            expires_at_ms: 1_050_000,
            is_revoked: false,
            token_id: Some(uuid::Uuid::new_v4()),
        };

        let req2 = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "knowledge.semantic_query",
                "arguments": { "query": "test" }
            }
        });
        let resp2 = service
            .handle_jsonrpc_with_context(req2, &auth_caller, 1_000_000)
            .await;
        assert!(resp2.error.is_none());
        let res_val = resp2.result.unwrap();
        assert_eq!(res_val["isError"], true);
        assert!(
            res_val["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("Thermal backpressure critical")
        );
    }
}
