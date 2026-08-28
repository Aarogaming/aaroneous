// Mutation Intent System
// Agents submit "Mutation Intent" packets instead of directly modifying state.
// The Single Writer validates intents like RNA polymerase verifying a cellular blueprint.

use crate::swmr_synapse::SynapseState;
use anyhow::Result;
use tokio::sync::mpsc;

/// A mutation intent packet - the "nucleotide sequence" requesting state change
#[derive(Debug, Clone)]
pub struct MutationIntent {
    /// Target field name (e.g., "curiosity_drive", "integrity_score")
    pub field_name: String,
    /// Raw bytes to write (interpreted based on field type)
    pub value: Vec<u8>,
    /// Optional reason/justification for the mutation
    pub reason: Option<String>,
}

impl MutationIntent {
    /// Apply this intent to a synapse state
    pub fn apply(&self, state: &mut SynapseState) -> Result<()> {
        match self.field_name.as_str() {
            "curiosity_drive" => {
                if self.value.len() == 1 {
                    state.curiosity_drive = self.value[0];
                }
            }
            "integrity_score" => {
                if self.value.len() == 1 {
                    state.integrity_score = self.value[0];
                }
            }
            "understanding_score" => {
                if self.value.len() == 1 {
                    state.understanding_score = self.value[0];
                }
            }
            "concept_drift" => {
                if self.value.len() == 4 {
                    state.concept_drift = f32::from_le_bytes([
                        self.value[0],
                        self.value[1],
                        self.value[2],
                        self.value[3],
                    ]);
                }
            }
            "energy_budget" => {
                if self.value.len() == 4 {
                    state.energy_budget = u32::from_le_bytes([
                        self.value[0],
                        self.value[1],
                        self.value[2],
                        self.value[3],
                    ]);
                }
            }
            "memory_pressure" => {
                if self.value.len() == 1 {
                    state.memory_pressure = self.value[0];
                }
            }
            "safety_lock" => {
                if self.value.len() == 1 {
                    state.safety_lock = self.value[0];
                }
            }
            "approval_required" => {
                if self.value.len() == 1 {
                    state.approval_required = self.value[0];
                }
            }
            "approval_granted" => {
                if self.value.len() == 1 {
                    state.approval_granted = self.value[0];
                }
            }
            "hox_mutation_flag" => {
                if self.value.len() == 1 {
                    state.hox_mutation_flag = self.value[0];
                }
            }
            "sovereignty_tier" => {
                if self.value.len() == 1 {
                    state.sovereignty_tier = self.value[0];
                }
            }
            "latent_vector" => {
                if self.value.len() == 1024 * 4 {
                    for i in 0..1024 {
                        let offset = i * 4;
                        state.latent_vector[i] = f32::from_le_bytes([
                            self.value[offset],
                            self.value[offset + 1],
                            self.value[offset + 2],
                            self.value[offset + 3],
                        ]);
                    }
                }
            }
            "mcp_status" => {
                if self.value.len() == 1 {
                    state.mcp_status = self.value[0];
                }
            }
            "dialogue_consensus" => {
                if self.value.len() == 1 {
                    state.dialogue_consensus = self.value[0];
                }
            }
            "shutdown" => {
                // Special intent to terminate writer loop
                return Err(anyhow::anyhow!("Shutdown requested"));
            }
            _ => {
                // Unknown field - ignore silently
            }
        }
        Ok(())
    }
}

/// Intent validator - the "RNA polymerase chain" that verifies mutation blueprints
#[derive(Clone)]
pub struct IntentValidator {
    /// Maximum allowed values for bounded fields
    max_values: std::collections::HashMap<String, u8>,
}

impl Default for IntentValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentValidator {
    pub fn new() -> Self {
        let mut max_values = std::collections::HashMap::new();
        max_values.insert("curiosity_drive".to_string(), 100);
        max_values.insert("integrity_score".to_string(), 100);
        max_values.insert("understanding_score".to_string(), 100);
        max_values.insert("memory_pressure".to_string(), 100);
        max_values.insert("safety_lock".to_string(), 1);
        max_values.insert("approval_required".to_string(), 1);
        max_values.insert("approval_granted".to_string(), 1);
        max_values.insert("hox_mutation_flag".to_string(), 1);
        max_values.insert("sovereignty_tier".to_string(), 2);
        max_values.insert("mcp_status".to_string(), 4);
        max_values.insert("dialogue_consensus".to_string(), 100);

        Self { max_values }
    }

    /// Validate a mutation intent against schema and constraints
    pub fn validate(&self, intent: &MutationIntent) -> bool {
        // Check field exists
        if !self.is_known_field(&intent.field_name) {
            return false;
        }

        // Check value bounds for u8 fields
        if let Some(&max) = self.max_values.get(&intent.field_name) {
            if intent.value.len() == 1 && intent.value[0] > max {
                return false;
            }
        }

        // Check size constraints for complex fields
        match intent.field_name.as_str() {
            "latent_vector" => {
                if intent.value.len() != 1024 * 4 {
                    return false;
                }
            }
            "concept_drift" | "energy_budget" => {
                if intent.value.len() != 4 {
                    return false;
                }
            }
            _ => {
                // Single byte fields
                if intent.value.len() > 1 && intent.field_name != "shutdown" {
                    return false;
                }
            }
        }

        true
    }

    fn is_known_field(&self, field: &str) -> bool {
        self.max_values.contains_key(field)
            || matches!(
                field,
                "latent_vector" | "concept_drift" | "energy_budget" | "shutdown"
            )
    }
}

/// Async queue for mutation intents
pub struct IntentQueue {
    rx: mpsc::UnboundedReceiver<MutationIntent>,
}

impl IntentQueue {
    pub fn new(rx: mpsc::UnboundedReceiver<MutationIntent>) -> Self {
        Self { rx }
    }

    pub async fn receive(&mut self) -> Option<MutationIntent> {
        self.rx.recv().await
    }

    pub fn try_receive(&mut self) -> Option<MutationIntent> {
        self.rx.try_recv().ok()
    }
}
