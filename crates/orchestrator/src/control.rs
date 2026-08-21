// Aaroneous Control Plane Module
// NATS federation control message handling and specialist lifecycle management

use crate::agents::{create_relic, create_specialist, RelicAgent, SpecialistAgent};
use biology::SystemBiology;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Control message types from federation bus
#[derive(Debug, Clone)]
pub enum ControlMessage {
    SpawnSpecialist {
        name: String,
        activate: bool,
        user_id: Option<String>,
    },
    HaltSpecialist {
        name: String,
    },
    SetExpressionRate {
        rate: f32,
    },
    RecalibrateSpecialist {
        name: String,
        bias: Option<serde_json::Value>,
    },
    AdjustResourceAllocation {
        specialist_name: String,
        vram_mb: u32,
        context_size: u32,
    },
    QuerySystemHealth,
    QuerySpecialistStatus {
        name: String,
    },
}

/// Parse incoming NATS message payload into ControlMessage
pub fn parse_control_message(payload: &str) -> Result<ControlMessage, String> {
    let json: Value = serde_json::from_str(payload).map_err(|e| format!("Invalid JSON: {}", e))?;

    match json.get("command").and_then(|c| c.as_str()) {
        Some("spawn_specialist") => {
            let name = json
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or("Missing 'name' field")?
                .to_string();
            let activate = json
                .get("activate")
                .and_then(|a| a.as_bool())
                .unwrap_or(true);
            let user_id = json
                .get("user_id")
                .and_then(|u| u.as_str())
                .map(|s| s.to_string());
            Ok(ControlMessage::SpawnSpecialist {
                name,
                activate,
                user_id,
            })
        }
        Some("halt_specialist") => {
            let name = json
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or("Missing 'name' field")?
                .to_string();
            Ok(ControlMessage::HaltSpecialist { name })
        }
        Some("set_expression_rate") => {
            let rate = json
                .get("rate")
                .and_then(|r| r.as_f64())
                .ok_or("Missing or invalid 'rate' field")? as f32;
            Ok(ControlMessage::SetExpressionRate { rate })
        }
        Some("recalibrate_specialist") => {
            let name = json
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or("Missing 'name' field")?
                .to_string();
            let bias = json.get("bias").cloned();
            Ok(ControlMessage::RecalibrateSpecialist { name, bias })
        }
        Some("adjust_resource_allocation") => {
            let specialist_name = json
                .get("specialist_name")
                .and_then(|s| s.as_str())
                .ok_or("Missing 'specialist_name' field")?
                .to_string();
            let vram_mb = json
                .get("vram_mb")
                .and_then(|v| v.as_u64())
                .ok_or("Missing or invalid 'vram_mb' field")? as u32;
            let context_size =
                json.get("context_size")
                    .and_then(|c| c.as_u64())
                    .ok_or("Missing or invalid 'context_size' field")? as u32;
            Ok(ControlMessage::AdjustResourceAllocation {
                specialist_name,
                vram_mb,
                context_size,
            })
        }
        Some("query_system_health") => Ok(ControlMessage::QuerySystemHealth),
        Some("query_specialist_status") => {
            let name = json
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or("Missing 'name' field")?
                .to_string();
            Ok(ControlMessage::QuerySpecialistStatus { name })
        }
        _ => Err("Unknown command or missing 'command' field".to_string()),
    }
}

/// Specialist state tracker
#[derive(Debug, Clone)]
pub struct SpecialistState {
    pub agent: SpecialistAgent,
    pub relic: Option<RelicAgent>,
    pub is_active: bool,
    pub task_handle: Option<String>, // Token/ID for the tokio task
    pub spawned_at: Option<String>,  // ISO 8601 timestamp
    pub execution_count: u64,
    pub error_count: u64,
}

/// Control plane for managing specialist lifecycle
pub struct ControlPlane {
    pub specialist_states: Arc<RwLock<HashMap<String, SpecialistState>>>,
    pub pending_commands: Arc<RwLock<Vec<ControlMessage>>>,
}

impl Default for ControlPlane {
    fn default() -> Self {
        Self::new()
    }
}

impl ControlPlane {
    pub fn new() -> Self {
        ControlPlane {
            specialist_states: Arc::new(RwLock::new(HashMap::new())),
            pending_commands: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Queue a control message for processing
    pub async fn enqueue_command(&self, msg: ControlMessage) {
        let mut commands = self.pending_commands.write().await;
        commands.push(msg);
    }

    /// Process all pending commands (call from main loop)
    pub async fn process_pending_commands(
        &self,
        biology: &mut SystemBiology,
    ) -> Vec<(String, Value)> {
        let mut commands = self.pending_commands.write().await;
        let mut responses = Vec::new();

        while let Some(cmd) = commands.pop() {
            let response = self.execute_command(cmd, biology).await;
            responses.push(response);
        }

        responses
    }

    /// Execute a single control command
    async fn execute_command(
        &self,
        msg: ControlMessage,
        biology: &mut SystemBiology,
    ) -> (String, Value) {
        match msg {
            ControlMessage::SpawnSpecialist {
                name,
                activate,
                user_id,
            } => {
                self.spawn_specialist(&name, activate, user_id, biology)
                    .await
            }
            ControlMessage::HaltSpecialist { name } => self.halt_specialist(&name).await,
            ControlMessage::SetExpressionRate { rate } => {
                biology.set_expression_rate(rate);
                (
                    "federation.control.response.set_expression_rate".to_string(),
                    json!({
                        "success": true,
                        "new_rate": rate,
                        "throttle_state": format!("{:?}", biology.throttle_state)
                    }),
                )
            }
            ControlMessage::RecalibrateSpecialist { name, bias } => {
                self.recalibrate_specialist(&name, bias).await
            }
            ControlMessage::AdjustResourceAllocation {
                specialist_name,
                vram_mb,
                context_size,
            } => {
                self.adjust_resource_allocation(&specialist_name, vram_mb, context_size)
                    .await
            }
            ControlMessage::QuerySystemHealth => {
                let health = biology.get_health_report();
                (
                    "federation.control.response.system_health".to_string(),
                    json!({
                        "global_tokens": health.global_tokens,
                        "expression_rate": health.expression_rate,
                        "throttle_state": format!("{:?}", health.throttle_state),
                        "specialist_count": health.specialist_count,
                        "specialists": health.specialist_health.iter().map(|s| json!({
                            "id": s.specialist_id,
                            "tokens": s.tokens,
                            "max_tokens": s.max_tokens,
                            "execution_count": s.execution_count,
                            "availability": s.token_availability
                        })).collect::<Vec<_>>()
                    }),
                )
            }
            ControlMessage::QuerySpecialistStatus { name } => {
                self.query_specialist_status(&name).await
            }
        }
    }

    /// Spawn a specialist and optionally its relic
    async fn spawn_specialist(
        &self,
        name: &str,
        activate: bool,
        user_id: Option<String>,
        biology: &mut SystemBiology,
    ) -> (String, Value) {
        let mut states = self.specialist_states.write().await;

        // Check if already exists
        if states.contains_key(name) {
            return (
                "federation.control.response.spawn_specialist".to_string(),
                json!({
                    "success": false,
                    "error": "Specialist already spawned",
                    "specialist": name
                }),
            );
        }

        // Create specialist
        let specialist = match create_specialist(name) {
            Some(s) => s,
            None => {
                return (
                    "federation.control.response.spawn_specialist".to_string(),
                    json!({
                        "success": false,
                        "error": "Unknown specialist",
                        "specialist": name
                    }),
                )
            }
        };

        // Create relic
        let relic_id = &specialist.supervised_relic.clone().unwrap_or_default();
        let relic = if let Some(r) = specialist.supervised_relic.clone() {
            create_relic(&r, &specialist.id)
        } else {
            None
        };

        // Register in metabolism
        biology.register_specialist(&specialist.id, specialist.interval_ms);

        // Create state
        let now = chrono::Local::now().to_rfc3339();
        let state = SpecialistState {
            agent: specialist.clone(),
            relic: relic.clone(),
            is_active: activate,
            task_handle: Some(format!("task_{}", uuid::Uuid::new_v4())),
            spawned_at: Some(now),
            execution_count: 0,
            error_count: 0,
        };

        states.insert(name.to_string(), state);

        (
            format!("federation.specialist.{}.spawned", name),
            json!({
                "success": true,
                "specialist": name,
                "relic": relic_id,
                "user_id": user_id,
                "active": activate,
                "persona": specialist.persona,
                "domain": format!("{:?}", specialist.domain)
            }),
        )
    }

    /// Halt a specialist and its supervised relic
    async fn halt_specialist(&self, name: &str) -> (String, Value) {
        let mut states = self.specialist_states.write().await;

        if let Some(state) = states.get_mut(name) {
            state.is_active = false;
            (
                format!("federation.specialist.{}.halted", name),
                json!({
                    "success": true,
                    "specialist": name,
                    "execution_count": state.execution_count,
                    "error_count": state.error_count
                }),
            )
        } else {
            (
                format!("federation.specialist.{}.halted", name),
                json!({
                    "success": false,
                    "error": "Specialist not found",
                    "specialist": name
                }),
            )
        }
    }

    /// Recalibrate specialist epigenetics
    async fn recalibrate_specialist(&self, name: &str, bias: Option<Value>) -> (String, Value) {
        let mut states = self.specialist_states.write().await;

        if let Some(state) = states.get_mut(name) {
            if let Some(bias_obj) = bias {
                // Parse bias from JSON
                if let Ok(updated_bias) = serde_json::from_value(bias_obj.clone()) {
                    state.agent.cognitive_bias = updated_bias;
                    return (
                        format!("federation.specialist.{}.recalibrated", name),
                        json!({
                            "success": true,
                            "specialist": name,
                            "new_bias": bias_obj
                        }),
                    );
                }
            }
            (
                format!("federation.specialist.{}.recalibrated", name),
                json!({
                    "success": true,
                    "specialist": name,
                    "message": "Bias updated (or not provided)"
                }),
            )
        } else {
            (
                format!("federation.specialist.{}.recalibrated", name),
                json!({
                    "success": false,
                    "error": "Specialist not found",
                    "specialist": name
                }),
            )
        }
    }

    /// Adjust resource allocation for a specialist
    async fn adjust_resource_allocation(
        &self,
        specialist_name: &str,
        vram_mb: u32,
        context_size: u32,
    ) -> (String, Value) {
        let states = self.specialist_states.read().await;

        if states.contains_key(specialist_name) {
            (
                "federation.control.response.adjust_resource".to_string(),
                json!({
                    "success": true,
                    "specialist": specialist_name,
                    "vram_mb": vram_mb,
                    "context_size": context_size
                }),
            )
        } else {
            (
                "federation.control.response.adjust_resource".to_string(),
                json!({
                    "success": false,
                    "error": "Specialist not found",
                    "specialist": specialist_name
                }),
            )
        }
    }

    /// Query status of a specialist
    async fn query_specialist_status(&self, name: &str) -> (String, Value) {
        let states = self.specialist_states.read().await;

        if let Some(state) = states.get(name) {
            (
                format!("federation.specialist.{}.status", name),
                json!({
                    "success": true,
                    "specialist": name,
                    "is_active": state.is_active,
                    "execution_count": state.execution_count,
                    "error_count": state.error_count,
                    "spawned_at": state.spawned_at,
                    "relic": state.relic.as_ref().map(|r| &r.name)
                }),
            )
        } else {
            (
                format!("federation.specialist.{}.status", name),
                json!({
                    "success": false,
                    "error": "Specialist not found",
                    "specialist": name
                }),
            )
        }
    }

    /// Get all active specialists
    pub async fn get_active_specialists(&self) -> Vec<String> {
        let states = self.specialist_states.read().await;
        states
            .iter()
            .filter(|(_, state)| state.is_active)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get specialist state
    pub async fn get_specialist_state(&self, name: &str) -> Option<SpecialistState> {
        let states = self.specialist_states.read().await;
        states.get(name).cloned()
    }
}

// Add necessary dependencies for dates/UUIDs (should be in Cargo.toml)
// chrono = "0.4"
// uuid = { version = "1.0", features = ["v4", "serde"] }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_spawn_specialist_command() {
        let json = r#"{"command": "spawn_specialist", "name": "ariel", "activate": true}"#;
        let msg = parse_control_message(json).unwrap();
        match msg {
            ControlMessage::SpawnSpecialist { name, activate, .. } => {
                assert_eq!(name, "ariel");
                assert!(activate);
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_parse_set_expression_rate_command() {
        let json = r#"{"command": "set_expression_rate", "rate": 0.5}"#;
        let msg = parse_control_message(json).unwrap();
        match msg {
            ControlMessage::SetExpressionRate { rate } => {
                assert_eq!(rate, 0.5);
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_parse_invalid_command() {
        let json = r#"{"command": "unknown"}"#;
        let result = parse_control_message(json);
        assert!(result.is_err());
    }
}
