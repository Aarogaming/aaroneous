wit_bindgen::generate!({
    world: "resource-governor",
});

use crate::exports::aaroneous::agent::specialist::Guest;
use crate::aaroneous::agent::bus;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
struct LoadTelemetry {
    node_id: String,
    cpu_usage: f32,
    memory_usage: f32,
    active_requests: u32,
}

#[derive(Deserialize, Serialize, Debug)]
struct ActionEvent {
    action: String,
    target_node: String,
    reason: String,
}

struct ResourceGovernor;

impl Guest for ResourceGovernor {
    fn process() -> Result<String, String> {
        // Subscribe to node telemetry
        bus::subscribe("system/telemetry/load").map_err(|e| format!("{:?}", e))?;

        let mut scaled_nodes = 0;
        
        // Attempt to receive a batch of messages non-blocking (or simply process one if blocking)
        if let Ok((topic, payload)) = bus::receive() {
            if topic == "system/telemetry/load" {
                if let Ok(telemetry) = serde_json::from_slice::<LoadTelemetry>(&payload) {
                    // Extremely basic scale logic
                    if telemetry.cpu_usage > 0.85 || telemetry.memory_usage > 0.90 {
                        let action = ActionEvent {
                            action: "SCALE_UP".to_string(),
                            target_node: telemetry.node_id.clone(),
                            reason: format!("High load detected. CPU: {:.2}, Mem: {:.2}", telemetry.cpu_usage, telemetry.memory_usage),
                        };
                        let action_payload = serde_json::to_vec(&action).unwrap();
                        bus::publish("system/actions/scale", &action_payload).map_err(|e| format!("{:?}", e))?;
                        scaled_nodes += 1;
                    } else if telemetry.cpu_usage < 0.20 && telemetry.active_requests < 10 {
                        let action = ActionEvent {
                            action: "SCALE_DOWN".to_string(),
                            target_node: telemetry.node_id.clone(),
                            reason: format!("Low load detected. CPU: {:.2}, Req: {}", telemetry.cpu_usage, telemetry.active_requests),
                        };
                        let action_payload = serde_json::to_vec(&action).unwrap();
                        bus::publish("system/actions/scale", &action_payload).map_err(|e| format!("{:?}", e))?;
                        scaled_nodes += 1;
                    }
                }
            }
        }
        
        Ok(format!("Processed telemetry. Executed {} scale actions.", scaled_nodes))
    }
}

export!(ResourceGovernor);