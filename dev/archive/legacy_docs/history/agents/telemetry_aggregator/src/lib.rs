wit_bindgen::generate!({
    world: "telemetry-aggregator",
});

use crate::exports::aaroneous::agent::specialist::Guest;
use crate::aaroneous::agent::bus;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
struct NodeHealth {
    node_id: String,
    status: String, // "healthy", "degraded", "offline"
    uptime_seconds: u64,
}

#[derive(Deserialize, Serialize, Debug)]
struct ClusterDigest {
    timestamp: u64,
    total_nodes: u32,
    healthy_nodes: u32,
    degraded_nodes: u32,
    offline_nodes: u32,
}

struct TelemetryAggregator;

impl Guest for TelemetryAggregator {
    fn process() -> Result<String, String> {
        bus::subscribe("system/nodes/health").map_err(|e| format!("{:?}", e))?;

        let mut healthy = 0;
        let mut degraded = 0;
        let mut offline = 0;
        let mut total = 0;
        
        // This would typically aggregate over a time window, but we simulate a single batch
        while let Ok((topic, payload)) = bus::receive() {
            if topic == "system/nodes/health" {
                 if let Ok(health) = serde_json::from_slice::<NodeHealth>(&payload) {
                     total += 1;
                     match health.status.as_str() {
                         "healthy" => healthy += 1,
                         "degraded" => degraded += 1,
                         "offline" => offline += 1,
                         _ => {}
                     }
                 }
            }
            // In a real system, you'd break when there are no more immediate messages
            break; 
        }
        
        if total > 0 {
            let digest = ClusterDigest {
                timestamp: 0, // We would use system time if available in the WASM env
                total_nodes: total,
                healthy_nodes: healthy,
                degraded_nodes: degraded,
                offline_nodes: offline,
            };
            
            let digest_payload = serde_json::to_vec(&digest).unwrap();
            bus::publish("system/cluster/digest", &digest_payload).map_err(|e| format!("{:?}", e))?;
            return Ok(format!("Published cluster digest for {} nodes.", total));
        }

        Ok("No new health telemetry to aggregate.".to_string())
    }
}

export!(TelemetryAggregator);