wit_bindgen::generate!({
    world: "genesis-architect",
});

use crate::exports::aaroneous::agent::specialist::Guest;
use crate::aaroneous::agent::bus;
use serde::{Deserialize, Serialize};

mod sdk {
    use crate::aaroneous::agent::bus;
    use serde::Serialize;

    pub fn log(level: &str, message: &str) {
        let payload = format!("{{\"level\":\"{}\", \"message\":\"{}\"}}", level, message);
        let _ = bus::publish("system/agent/log", payload.as_bytes());
    }

    pub fn emit_telemetry<T: Serialize>(agent_name: &str, data: &T) {
        if let Ok(payload) = serde_json::to_vec(data) {
            let topic = format!("system/agent/telemetry/{}", agent_name);
            let _ = bus::publish(&topic, &payload);
        }
    }
}

// --- Omni Constellation Data Structures (for parsing responses) ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniVector {
    pub dimensions: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniNode {
    pub id: String,
    pub title: String,
    pub domain: String,
    pub content: String,
    pub coordinates: OmniVector,
    pub mass: f32,
}

// --- Bus Payloads ---

#[derive(Deserialize, Debug)]
struct DeployRequest {
    target_hardware: String, // e.g. "raspberry_pi", "server", "mobile"
    mission_purpose: String, // e.g. "network_monitoring", "spatial_rendering"
    request_id: Option<String>,
}

#[derive(Serialize, Debug)]
struct QueryPayload {
    origin: Vec<f32>,
    radius: f32,
    max_results: usize,
    request_id: String,
}

#[derive(Deserialize, Debug)]
struct QueryResult {
    request_id: String,
    nodes: Vec<OmniNode>,
}

#[derive(Serialize, Debug)]
struct BuildCommand {
    target: String,
    modules: Vec<String>,
    knowledge_subset_ids: Vec<String>,
    job_id: String,
}

#[derive(Serialize)]
struct AgentTelemetry {
    deploy_requests_handled: u32,
    build_commands_issued: u32,
}

// --- Agent State ---
static mut DEPLOYS_HANDLED: u32 = 0;
static mut BUILDS_ISSUED: u32 = 0;

// To handle asynchronous multi-step workflows in a simple cycle
static mut PENDING_DEPLOYS: Option<DeployRequest> = None;

struct GenesisArchitect;

impl Guest for GenesisArchitect {
    fn process() -> Result<String, String> {
        sdk::log("INFO", "GenesisArchitect cycle started.");
        
        bus::subscribe("system/deploy/request").map_err(|e| format!("{:?}", e))?;
        bus::subscribe("system/knowledge/results").map_err(|e| format!("{:?}", e))?;

        let mut processed = 0;

        while let Ok((topic, payload)) = bus::receive() {
            match topic.as_str() {
                "system/deploy/request" => {
                    if let Ok(req) = serde_json::from_slice::<DeployRequest>(&payload) {
                        sdk::log("INFO", &format!("Received deployment request for target: {}, mission: {}", req.target_hardware, req.mission_purpose));
                        
                        // 1. Translate mission into an Omni vector query (Simulated deterministic vector mapping)
                        // In reality, this would use a small local embedding model or ask the Visionary to embed the string
                        let mut origin = vec![0.0; 256];
                        if req.mission_purpose.contains("security") || req.mission_purpose.contains("monitor") {
                            origin[0] = 1.0; origin[10] = 0.8;
                        } else if req.mission_purpose.contains("spatial") || req.mission_purpose.contains("ar") {
                            origin[5] = 1.0; origin[15] = 0.9;
                        } else {
                            origin[128] = 1.0;
                        }

                        let query_id = req.request_id.clone().unwrap_or_else(|| "gen-q-1".to_string());
                        
                        let query = QueryPayload {
                            origin,
                            radius: 0.5,
                            max_results: 5,
                            request_id: query_id,
                        };

                        if let Ok(q_payload) = serde_json::to_vec(&query) {
                            bus::publish("system/knowledge/query", &q_payload).map_err(|e| format!("{:?}", e))?;
                            sdk::log("INFO", "Dispatched query to OmniRelic for genetic blueprint.");
                        }

                        // Store pending state
                        unsafe { PENDING_DEPLOYS = Some(req); DEPLOYS_HANDLED += 1; }
                    }
                }
                "system/knowledge/results" => {
                    if let Ok(results) = serde_json::from_slice::<QueryResult>(&payload) {
                        // 2. We received the genetic blueprint from OmniRelic
                        sdk::log("INFO", &format!("Received OmniRelic blueprint with {} knowledge nodes.", results.nodes.len()));
                        
                        if let Some(pending) = unsafe { PENDING_DEPLOYS.take() } {
                            // 3. Formulate the BuildCommand for the Core BootstrapSystem
                            let mut modules = vec!["Sentinel".to_string(), "Omnipresent".to_string()]; // Always include core and networking
                            
                            // Map target/mission to modules based on Omni knowledge (simulated mapping)
                            if pending.mission_purpose.contains("security") {
                                modules.push("SecuritySentinel".to_string());
                            } else if pending.mission_purpose.contains("spatial") {
                                modules.push("Phygital".to_string());
                            }
                            
                            let subset_ids: Vec<String> = results.nodes.into_iter().map(|n| n.id).collect();

                            let build_cmd = BuildCommand {
                                target: pending.target_hardware,
                                modules,
                                knowledge_subset_ids: subset_ids,
                                job_id: format!("build-{}", uuid::Uuid::new_v4()),
                            };

                            if let Ok(cmd_payload) = serde_json::to_vec(&build_cmd) {
                                bus::publish("system/deploy/build", &cmd_payload).map_err(|e| format!("{:?}", e))?;
                                sdk::log("WARN", &format!("Issued CORE BUILD COMMAND. Job ID: {}. Triggering Sovereign Packaging.", build_cmd.job_id));
                                unsafe { BUILDS_ISSUED += 1; }
                            }
                        }
                    }
                }
                _ => {}
            }
            processed += 1;
            break; 
        }

        let (deploys, builds) = unsafe { (DEPLOYS_HANDLED, BUILDS_ISSUED) };
        sdk::emit_telemetry("genesis-architect", &AgentTelemetry {
            deploy_requests_handled: deploys,
            build_commands_issued: builds,
        });

        Ok(format!("Genesis cycle complete. Processed {} messages.", processed))
    }
}

export!(GenesisArchitect);