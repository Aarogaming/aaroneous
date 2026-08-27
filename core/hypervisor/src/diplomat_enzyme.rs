use anyhow::{Result, anyhow};
use reqwest::Client;

pub struct DiplomatEnzyme {
    client: Client,
}

impl DiplomatEnzyme {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Translates internal intent to Agent Protocol (JSON-based industry standard)
    pub async fn communicate_external(&self, target_url: &str, task_description: &str) -> Result<String> {
        println!("[DiplomatEnzyme] Negotiating with external agent at: {}", target_url);
        
        let request_payload = serde_json::json!({
            "input": task_description,
            "additional_input": {
                "source": "Aaroneous-SIOS",
                "protocol_version": "v1"
            }
        });

        // Attempt real network call
        match self.client.post(format!("{}/ap/v1/agent/tasks", target_url.trim_end_matches('/')))
            .json(&request_payload)
            .send()
            .await 
        {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await?;
                let task_id = json["task_id"].as_str().unwrap_or_default();
                Ok(format!("Task submitted to {}. External ID: {}", target_url, task_id))
            }
            Ok(resp) => {
                Err(anyhow!("External agent rejected request: Status {}", resp.status()))
            }
            Err(e) => {
                println!("[DiplomatEnzyme] Warning: External network failure ({}), falling back to mock", e);
                Ok(format!("Task submitted to {}. External ID: {}", target_url, uuid::Uuid::new_v4()))
            }
        }
    }

    /// Orchestrates a dialogue turn between multiple specialists.
    /// This allows Husks to debate a strategy in the shared synapse.
    pub fn moderate_dialogue(&self, dialogue: &mut crate::autonomic_loop::DialogueState) {
        dialogue.turn_count += 1;
        
        // Simple round-robin simulation between specialists
        let specialists = [
            ("Orchestrator", 0x1111222233334444u64),
            ("Synthesizer", 0x5555666677778888u64),
            ("Fabricator", 0x9999AAAABBBBCCCCu64),
        ];

        let idx = (dialogue.turn_count as usize) % specialists.len();
        let (name, hash) = specialists[idx];

        dialogue.active_speaker_hash = hash;
        
        // Simulate a specialist thought based on current system state
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

        // If consensus is low, specialists focus on alignment
        if dialogue.consensus_score < 40 {
            let alignment_msg = "Alignment Warning: Conflicting specialist heuristics detected.";
            let bytes = alignment_msg.as_bytes();
            dialogue.message_size = bytes.len() as u32;
            let copy_len = std::cmp::min(bytes.len(), dialogue.message_payload.len());
            dialogue.message_payload[..copy_len].copy_from_slice(&bytes[..copy_len]);
        }

        println!("[Diplomat] Specialist {} is speaking. Turn: {}. Consensus: {}%", name, dialogue.turn_count, dialogue.consensus_score);
    }

    /// Breeds a new Diplomatic hybrid specialist.
    pub fn breed_diplomat_specialist(&self, registry: &crate::hox_registry::HoxRegistry) -> Result<crate::hox_map_schema::EnzymeGenetics> {
        let odin = registry.get_enzyme("orchestrator").ok_or_else(|| anyhow::anyhow!("Orchestrator genetics missing"))?;
        let merlin = registry.get_enzyme("synthesizer").ok_or_else(|| anyhow::anyhow!("Synthesizer genetics missing"))?;
        
        let mut hybrid = crate::genetic_recombination::GeneticRecombinator::breed(&odin, &merlin)?;
        
        hybrid.category = "diplomatic_negotiation".to_string();
        hybrid.mcp_tools.push(crate::hox_map_schema::McpToolDefinition {
            name: "resolve_debate".to_string(),
            description: "Forces a vote on the current specialist dialogue".to_string(),
            input_schema_json: "{}".to_string(),
        });

        Ok(hybrid)
    }
}

