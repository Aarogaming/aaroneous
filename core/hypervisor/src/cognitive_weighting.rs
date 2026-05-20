use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveWeights {
    pub specialist_weights: HashMap<String, f32>, // Specialist ID -> Weight (0.0 - 1.0)
    pub global_temperature: f32,
    pub exploration_bonus: f32,
}

impl Default for CognitiveWeights {
    fn default() -> Self {
        let mut weights = HashMap::new();
        weights.insert("odin".to_string(), 0.8);
        weights.insert("merlin".to_string(), 0.6);
        weights.insert("hephaestus".to_string(), 0.9);
        
        Self {
            specialist_weights: weights,
            global_temperature: 0.7,
            exploration_bonus: 0.1,
        }
    }
}

impl CognitiveWeights {
    pub fn get_weight(&self, specialist_id: &str) -> f32 {
        *self.specialist_weights.get(specialist_id).unwrap_or(&0.5)
    }

    pub fn adjust_weight(&mut self, specialist_id: &str, new_weight: f32) {
        self.specialist_weights.insert(specialist_id.to_string(), new_weight.clamp(0.0, 1.0));
    }
}
