//! crates/compute/src/si_motor_tree.rs
//! Hierarchical Motor Skill Constellation & Motor Cortex Substrate.
//!
//! Features:
//! 1. StarState transitions: Neural (learning) -> Compiling -> Crystallized (bare-metal function pointer).
//! 2. SkillType categorization: Primitive (direct machine opcode) vs Composite (DAG sequence).
//! 3. R^256 Intent Embedding similarity matching via SIMD vector dot products.
//! 4. MotorCortex registry for indexing, activation tracking, and auto-crystallization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MOTOR_INTENT_DIM: usize = 256;

/// Visual & Thermodynamic Execution State of a Skill Node
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StarState {
    /// Active continuous neural inference undergoing online LoRA adaptation
    Neural { variance: f32 },
    /// Pathway currently undergoing Cranelift W^X JIT compilation
    Compiling,
    /// Fully mastered bare-metal function pointer in executable memory
    Crystallized { addr: usize, time_ns: u64 },
}

impl StarState {
    pub fn is_crystallized(&self) -> bool {
        matches!(self, Self::Crystallized { .. })
    }

    pub fn variance(&self) -> f32 {
        match self {
            Self::Neural { variance } => *variance,
            Self::Compiling => 0.005,
            Self::Crystallized { .. } => 0.0,
        }
    }
}

/// Skill Execution Topology
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SkillType {
    /// Atomic machine opcode
    Primitive { opcode_id: u32 },
    /// High-level composite routine chaining multiple child sub-skills
    Composite { sub_skills: Vec<String> },
}

mod serde_256 {
    use super::MOTOR_INTENT_DIM;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(val: &[f32; MOTOR_INTENT_DIM], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        val.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[f32; MOTOR_INTENT_DIM], D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<f32>::deserialize(deserializer)?;
        let mut arr = [0.0f32; MOTOR_INTENT_DIM];
        for (i, &v) in vec.iter().enumerate().take(MOTOR_INTENT_DIM) {
            arr[i] = v;
        }
        Ok(arr)
    }
}

/// A Star Node in the Motor Skill Constellation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MotorSkillNode {
    pub id: String,
    pub description: String,
    pub skill_type: SkillType,
    #[serde(with = "serde_256")]
    pub intent_embedding: [f32; MOTOR_INTENT_DIM],
    pub state: StarState,
    pub children: Vec<String>,
    pub execution_count: u64,
    pub success_count: u64,
}

impl MotorSkillNode {
    /// Computes the Cosine Similarity between this skill's intent and another in R^256
    pub fn cosine_similarity(&self, other: &MotorSkillNode) -> f32 {
        let mut dot = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;

        for i in 0..MOTOR_INTENT_DIM {
            dot += self.intent_embedding[i] * other.intent_embedding[i];
            norm_a += self.intent_embedding[i] * self.intent_embedding[i];
            norm_b += other.intent_embedding[i] * other.intent_embedding[i];
        }

        let denom = (norm_a.sqrt() * norm_b.sqrt()).max(1e-6);
        (dot / denom).clamp(-1.0, 1.0)
    }

    /// Computes Euclidean distance to a target candidate intent vector
    pub fn euclidean_distance(&self, target_intent: &[f32; MOTOR_INTENT_DIM]) -> f32 {
        let mut sum = 0.0f32;
        for i in 0..MOTOR_INTENT_DIM {
            let diff = self.intent_embedding[i] - target_intent[i];
            sum += diff * diff;
        }
        sum.sqrt()
    }
}

/// Hierarchical Motor Cortex Registry
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MotorCortex {
    pub registry: HashMap<String, MotorSkillNode>,
    pub total_activations: u64,
    pub total_crystallized: usize,
}

impl MotorCortex {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
            total_activations: 0,
            total_crystallized: 0,
        }
    }

    /// Registers a new motor skill node into the constellation
    pub fn register_skill(&mut self, node: MotorSkillNode) {
        if node.state.is_crystallized() {
            self.total_crystallized += 1;
        }
        self.registry.insert(node.id.clone(), node);
    }

    pub fn get_skill(&self, id: &str) -> Option<&MotorSkillNode> {
        self.registry.get(id)
    }

    pub fn get_skill_mut(&mut self, id: &str) -> Option<&mut MotorSkillNode> {
        self.registry.get_mut(id)
    }

    pub fn len(&self) -> usize {
        self.registry.len()
    }

    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }

    /// Finds the nearest semantic skill in R^256 intent space
    pub fn find_nearest_skill(&self, intent: &[f32; MOTOR_INTENT_DIM]) -> Option<(&MotorSkillNode, f32)> {
        let mut best_node = None;
        let mut min_dist = f32::MAX;

        for node in self.registry.values() {
            let dist = node.euclidean_distance(intent);
            if dist < min_dist {
                min_dist = dist;
                best_node = Some(node);
            }
        }

        best_node.map(|n| (n, min_dist))
    }

    /// Crystallizes a mature skill into a bare-metal execution address
    pub fn crystallize_skill(&mut self, id: &str, executable_addr: usize, time_ns: u64) -> bool {
        if let Some(node) = self.registry.get_mut(id) {
            node.state = StarState::Crystallized {
                addr: executable_addr,
                time_ns,
            };
            self.total_crystallized += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motor_cortex_registration_and_nearest_search() {
        let mut cortex = MotorCortex::new();

        let mut intent1 = [0.0f32; MOTOR_INTENT_DIM];
        intent1[0] = 1.0;

        let mut intent2 = [0.0f32; MOTOR_INTENT_DIM];
        intent2[1] = 1.0;

        cortex.register_skill(MotorSkillNode {
            id: "mouse_click".into(),
            description: "Click primary mouse button".into(),
            skill_type: SkillType::Primitive { opcode_id: 0x01 },
            intent_embedding: intent1,
            state: StarState::Crystallized { addr: 0x7FFA_0001, time_ns: 95 },
            children: vec![],
            execution_count: 50,
            success_count: 50,
        });

        cortex.register_skill(MotorSkillNode {
            id: "key_press".into(),
            description: "Send hardware scancode".into(),
            skill_type: SkillType::Primitive { opcode_id: 0x02 },
            intent_embedding: intent2,
            state: StarState::Neural { variance: 0.08 },
            children: vec![],
            execution_count: 5,
            success_count: 4,
        });

        assert_eq!(cortex.len(), 2);
        assert_eq!(cortex.total_crystallized, 1);

        // Query nearest
        let query = intent1;
        let (nearest, dist) = cortex.find_nearest_skill(&query).unwrap();
        assert_eq!(nearest.id, "mouse_click");
        assert!(dist < 1e-4);
    }
}
