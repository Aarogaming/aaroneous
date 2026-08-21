wit_bindgen::generate!({
    world: "omni-relic",
});

use crate::exports::aaroneous::agent::specialist::Guest;
use crate::aaroneous::agent::bus;
use crate::aaroneous::agent::llm;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

// --- Omni Constellation Data Structures ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniVector {
    pub dimensions: Vec<f32>,
}

impl OmniVector {
    pub fn new(dimensions: Vec<f32>) -> Self {
        Self { dimensions }
    }

    pub fn relativity_to(&self, other: &OmniVector) -> f32 {
        let dot: f32 = self.dimensions.iter().zip(other.dimensions.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = self.dimensions.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.dimensions.iter().map(|x| x * x).sum::<f32>().sqrt();
        let sim = if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) };
        1.0 - sim
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniNode {
    pub id: String,
    pub title: String,
    pub domain: String,
    pub content: String,
    pub coordinates: OmniVector,
    pub mass: f32,
    pub links: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniConstellation {
    pub name: String,
    pub nodes: HashMap<String, OmniNode>,
    pub dimensions: usize,
}

impl OmniConstellation {
    pub fn new(name: &str, dimensions: usize) -> Self {
        Self {
            name: name.to_string(),
            nodes: HashMap::new(),
            dimensions,
        }
    }

    pub fn inject(&mut self, title: &str, domain: &str, content: &str, dims: Vec<f32>, mass: f32) -> String {
        let mut dims = dims;
        while dims.len() < self.dimensions {
            dims.push(0.0);
        }
        dims.truncate(self.dimensions);

        // Generate simple ID (wasm32 doesn't easily support UUID without extra config)
        let id = format!("omni-{}-{}-{}", domain, title.len(), self.nodes.len());
        
        let mut node = OmniNode {
            id: id.clone(),
            title: title.to_string(),
            domain: domain.to_string(),
            content: content.to_string(),
            coordinates: OmniVector::new(dims),
            mass,
            links: HashMap::new(),
        };

        for (existing_id, existing_node) in self.nodes.iter_mut() {
            let distance = node.coordinates.relativity_to(&existing_node.coordinates);
            if distance < 0.3 {
                existing_node.links.insert(node.id.clone(), distance);
                node.links.insert(existing_id.clone(), distance);
            }
        }

        self.nodes.insert(id.clone(), node);
        id
    }

    pub fn query_relativistic(&self, origin: &OmniVector, radius: f32, max_results: usize) -> Vec<OmniNode> {
        let mut results: Vec<(&OmniNode, f32)> = self.nodes.values()
            .map(|n| (n, n.coordinates.relativity_to(origin)))
            .filter(|(_, dist)| *dist <= radius)
            .collect();

        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        results.into_iter()
            .take(max_results)
            .map(|(n, _)| n.clone())
            .collect()
    }
}

// --- Bus Payloads ---

#[derive(Deserialize)]
struct InjectPayload {
    title: String,
    domain: String,
    content: String,
    dimensions: Vec<f32>,
    mass: f32,
}

#[derive(Deserialize)]
struct QueryPayload {
    origin: Vec<f32>,
    radius: f32,
    max_results: usize,
    request_id: String,
}

#[derive(Serialize)]
struct QueryResult {
    request_id: String,
    nodes: Vec<OmniNode>,
}

#[derive(Serialize)]
struct AgentTelemetry {
    nodes_count: usize,
    injections_processed: u32,
    queries_processed: u32,
    crystallize_events: u32,
}

// --- Main Agent State ---

// In a real WASM scenario, we'd use a lazy_static or thread_local to persist state between process() calls
// For this prototype, we'll demonstrate the logic within a single process cycle.
static mut STATE_NODES: usize = 0;
static mut INJECTIONS: u32 = 0;
static mut QUERIES: u32 = 0;
static mut CRYSTALLIZATIONS: u32 = 0;

struct OmniRelic;

impl Guest for OmniRelic {
    fn process() -> Result<String, String> {
        sdk::log("INFO", "OmniRelic cycle started.");
        
        bus::subscribe("system/knowledge/inject").map_err(|e| format!("{:?}", e))?;
        bus::subscribe("system/knowledge/query").map_err(|e| format!("{:?}", e))?;

        // Instantiate local constellation (in a real stateful WASM, this is persistent)
        let mut constellation = OmniConstellation::new("hive-master", 256);
        
        let mut cycle_injections = 0;
        let mut cycle_queries = 0;

        while let Ok((topic, payload)) = bus::receive() {
            match topic.as_str() {
                "system/knowledge/inject" => {
                    if let Ok(data) = serde_json::from_slice::<InjectPayload>(&payload) {
                        let content_to_embed = format!("{} {} {}", data.domain, data.title, data.content);
                        let dimensions = if data.dimensions.iter().all(|&x| x == 0.0) || data.dimensions.is_empty() {
                            // If dimensions are empty or all zero, use the LLM to embed the content
                            match llm::embed(&content_to_embed) {
                                Ok(vec) => vec,
                                Err(_) => {
                                    sdk::log("WARN", "Embedding failed, falling back to provided or zeros");
                                    data.dimensions
                                }
                            }
                        } else {
                            data.dimensions
                        };

                        let id = constellation.inject(&data.title, &data.domain, &data.content, dimensions, data.mass);
                        sdk::log("INFO", &format!("Injected knowledge node: {}", id));
                        cycle_injections += 1;
                        unsafe { INJECTIONS += 1; STATE_NODES += 1; }
                    } else {
                        sdk::log("ERROR", "Failed to deserialize InjectPayload");
                    }
                }
                "system/knowledge/query" => {
                    if let Ok(data) = serde_json::from_slice::<QueryPayload>(&payload) {
                        // If origin is zero-filled, perhaps we embed something? QueryPayload doesn't have a string though.
                        let origin = OmniVector::new(data.origin);
                        let results = constellation.query_relativistic(&origin, data.radius, data.max_results);
                        
                        let response = QueryResult {
                            request_id: data.request_id,
                            nodes: results,
                        };
                        
                        if let Ok(res_payload) = serde_json::to_vec(&response) {
                            let _ = bus::publish("system/knowledge/results", &res_payload);
                            sdk::log("INFO", &format!("Answered query {}", response.request_id));
                        }
                        cycle_queries += 1;
                        unsafe { QUERIES += 1; }
                    } else {
                        sdk::log("ERROR", "Failed to deserialize QueryPayload");
                    }
                }
                _ => {}
            }
            break; // Process one message per tick to avoid locking
        }

        // Trigger crystallization logic periodically (e.g. every 10 injections)
        let total_injections = unsafe { INJECTIONS };
        let mut crystallized = false;
        
        if cycle_injections > 0 && total_injections % 10 == 0 {
            if let Ok(constellation_json) = serde_json::to_string(&constellation) {
                let _ = bus::publish("system/knowledge/crystallize", constellation_json.as_bytes());
                sdk::log("WARN", "Triggered GGUF metadata crystallization (Omni backup).");
                unsafe { CRYSTALLIZATIONS += 1; }
                crystallized = true;
            }
        }

        let (nodes, injs, qs, crysts) = unsafe { (STATE_NODES, INJECTIONS, QUERIES, CRYSTALLIZATIONS) };
        sdk::emit_telemetry("omni-relic", &AgentTelemetry {
            nodes_count: nodes,
            injections_processed: injs,
            queries_processed: qs,
            crystallize_events: crysts,
        });

        sdk::log("INFO", &format!("OmniRelic cycle finished. Injected: {}, Queried: {}", cycle_injections, cycle_queries));
        
        let mut out = format!("Processed {} injections, {} queries.", cycle_injections, cycle_queries);
        if crystallized {
            out.push_str(" Triggered crystallization.");
        }
        Ok(out)
    }
}

export!(OmniRelic);