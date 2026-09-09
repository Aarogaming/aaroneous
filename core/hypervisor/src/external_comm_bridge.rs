use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;

/// External Communication Bridge (formerly DiplomatEnzyme)
pub struct ExternalCommBridge {
    client: Client,
}

impl ExternalCommBridge {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    /// Translates internal intent to external agent protocol (JSON based).
    pub async fn communicate_external(&self, target_url: &str, task_description: &str) -> Result<String> {
        println!("[ExternalCommBridge] Negotiating with external agent at: {}", target_url);

        let request_payload = json!({
            "input": task_description,
            "additional_input": {
                "source": "Aaroneous-SIOS",
                "protocol_version": "v1"
            }
        });

        // Attempt real network call
        let response = self
            .client
            .post(format!("{}/ap/v1/agent/tasks", target_url.trim_end_matches('/')))
            .json(&request_payload)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await?;
                let task_id = json["task_id"].as_str().unwrap_or_default();
                Ok(format!("Task submitted to {}. External ID: {}", target_url, task_id))
            }
            Ok(resp) => Err(anyhow!(format!("External agent rejected request: Status {}", resp.status()))),
            Err(e) => Err(anyhow!(format!("External network failure communicating with {}: {}", target_url, e))),
        }
    }

    /// Orchestrates a dialogue turn between multiple specialists.
    pub fn moderate_dialogue(&self, dialogue: &mut crate::autonomic_loop::DialogueState) {
        dialogue.turn_count += 1;
        let specialists = [
            ("Orchestrator", 0x1111222233334444u64),
            ("Synthesizer", 0x5555666677778888u64),
            ("Fabricator", 0x9999AAAABBBBCCCCu64),
        ];
        let idx = (dialogue.turn_count as usize) % specialists.len();
        let (name, hash) = specialists[idx];
        dialogue.active_speaker_hash = hash;
        let thought = match name {
            "Orchestrator" => "Strategy: Focus on scaling the WASM isolation boundary.",
            "Synthesizer" => "Observation: Semantic drift is increasing in the latent subspace.",
            "Fabricator" => "Action: Optimizing the zero-copy synapse for lower latency.",
            _ => "...",
        };
        let bytes = thought.as_bytes();
        dialogue.message_size = bytes.len() as u32;
        let copy_len = std::cmp::min(bytes.len(), dialogue.message_payload.len());
        dialogue.message_payload[..copy_len].copy_from_slice(&bytes[..copy_len]);
        // Diplomacy: Slowly build consensus
        dialogue.consensus_score = (dialogue.consensus_score + 5).min(100);
        if dialogue.consensus_score < 40 {
            let alignment_msg = "Alignment Warning: Conflicting specialist heuristics detected.";
            let bytes = alignment_msg.as_bytes();
            dialogue.message_size = bytes.len() as u32;
            let copy_len = std::cmp::min(bytes.len(), dialogue.message_payload.len());
            dialogue.message_payload[..copy_len].copy_from_slice(&bytes[..copy_len]);
        }
        println!("[ExternalCommBridge] Specialist {} is speaking. Turn: {}. Consensus: {}%", name, dialogue.turn_count, dialogue.consensus_score);
    }
}
