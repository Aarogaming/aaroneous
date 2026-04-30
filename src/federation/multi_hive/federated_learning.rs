/// Federated Learning: Cross-Hive Model Improvement
/// 
/// Enables specialists to learn from gradients shared across hives

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Gradient update from a specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientUpdate {
    pub specialist_id: crate::federation::specialist::SpecialistId,
    pub update_id: String,
    pub round: u32,
    pub gradients: Vec<f32>,
    pub accuracy: f32,
    pub loss: f32,
    pub sample_count: u32,
    pub timestamp_ms: u64,
}

impl GradientUpdate {
    pub fn new(
        specialist_id: crate::federation::specialist::SpecialistId,
        gradients: Vec<f32>,
    ) -> Self {
        Self {
            specialist_id,
            update_id: uuid::Uuid::new_v4().to_string(),
            round: 0,
            gradients,
            accuracy: 0.0,
            loss: 1.0,
            sample_count: 0,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Average this gradient with another
    pub fn average_with(&self, other: &GradientUpdate) -> GradientUpdate {
        let mut avg_gradients = Vec::new();
        for (g1, g2) in self.gradients.iter().zip(other.gradients.iter()) {
            avg_gradients.push((g1 + g2) / 2.0);
        }

        GradientUpdate {
            specialist_id: self.specialist_id,
            update_id: uuid::Uuid::new_v4().to_string(),
            round: self.round + 1,
            gradients: avg_gradients,
            accuracy: (self.accuracy + other.accuracy) / 2.0,
            loss: (self.loss + other.loss) / 2.0,
            sample_count: self.sample_count + other.sample_count,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

/// Merged model from multiple specialists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedModel {
    pub model_id: String,
    pub specialist_id: crate::federation::specialist::SpecialistId,
    pub merged_from: Vec<String>,
    pub average_accuracy: f32,
    pub merged_gradients: Vec<f32>,
    pub num_contributors: u32,
    pub created_at_ms: u64,
}

/// Model merging strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMerger {
    pub merge_strategy: MergeStrategy,
    pub min_contributors: u32,
    pub max_gradient_diff: f32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Simple average of all gradients
    SimpleAverage,
    /// Weighted by accuracy
    AccuracyWeighted,
    /// Median of gradients
    Median,
    /// Federated Averaging (FedAvg)
    FederatedAverage,
}

impl ModelMerger {
    pub fn new() -> Self {
        Self {
            merge_strategy: MergeStrategy::FederatedAverage,
            min_contributors: 2,
            max_gradient_diff: 0.5,
        }
    }

    /// Merge multiple gradient updates
    pub fn merge(
        &self,
        updates: Vec<GradientUpdate>,
        specialist_id: crate::federation::specialist::SpecialistId,
    ) -> Result<MergedModel, String> {
        if updates.len() < self.min_contributors as usize {
            return Err(format!(
                "Not enough contributors: {} < {}",
                updates.len(),
                self.min_contributors
            ));
        }

        let merged_gradients = self.merge_gradients(&updates);
        let avg_accuracy = updates.iter().map(|u| u.accuracy).sum::<f32>() / updates.len() as f32;

        Ok(MergedModel {
            model_id: uuid::Uuid::new_v4().to_string(),
            specialist_id,
            merged_from: updates.iter().map(|u| u.update_id.clone()).collect(),
            average_accuracy: avg_accuracy,
            merged_gradients,
            num_contributors: updates.len() as u32,
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }

    /// Merge gradients using selected strategy
    fn merge_gradients(&self, updates: &[GradientUpdate]) -> Vec<f32> {
        if updates.is_empty() {
            return Vec::new();
        }

        let num_params = updates[0].gradients.len();
        let mut merged = vec![0.0; num_params];

        match self.merge_strategy {
            MergeStrategy::SimpleAverage => {
                for update in updates {
                    for (i, &g) in update.gradients.iter().enumerate() {
                        merged[i] += g;
                    }
                }
                for m in &mut merged {
                    *m /= updates.len() as f32;
                }
            }
            MergeStrategy::AccuracyWeighted => {
                let total_accuracy: f32 = updates.iter().map(|u| u.accuracy).sum();
                for update in updates {
                    let weight = update.accuracy / total_accuracy;
                    for (i, &g) in update.gradients.iter().enumerate() {
                        merged[i] += g * weight;
                    }
                }
            }
            MergeStrategy::Median => {
                for param_idx in 0..num_params {
                    let mut values: Vec<f32> =
                        updates.iter().map(|u| u.gradients[param_idx]).collect();
                    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    merged[param_idx] = values[values.len() / 2];
                }
            }
            MergeStrategy::FederatedAverage => {
                let total_samples: u32 = updates.iter().map(|u| u.sample_count).sum();
                for update in updates {
                    let weight = update.sample_count as f32 / total_samples as f32;
                    for (i, &g) in update.gradients.iter().enumerate() {
                        merged[i] += g * weight;
                    }
                }
            }
        }

        merged
    }
}

impl Default for ModelMerger {
    fn default() -> Self {
        Self::new()
    }
}

/// Federated learning engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedLearningEngine {
    pub rounds_completed: u32,
    pub models_trained: u32,
    pub total_gradients_exchanged: u64,
    pub global_accuracy: f32,
}

impl FederatedLearningEngine {
    pub fn new() -> Self {
        Self {
            rounds_completed: 0,
            models_trained: 0,
            total_gradients_exchanged: 0,
            global_accuracy: 0.5,
        }
    }

    /// Process a training round
    pub fn train_round(&mut self, gradients: Vec<GradientUpdate>) -> f32 {
        self.rounds_completed += 1;
        self.total_gradients_exchanged += gradients.len() as u64;

        // Update global accuracy (simple moving average)
        let avg_acc = gradients.iter().map(|g| g.accuracy).sum::<f32>() / gradients.len() as f32;
        self.global_accuracy = (self.global_accuracy * 0.8) + (avg_acc * 0.2);

        self.global_accuracy
    }
}

impl Default for FederatedLearningEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_update_creation() {
        let update = GradientUpdate::new(
            crate::federation::specialist::SpecialistId::Visionary,
            vec![0.1, 0.2, 0.3],
        );
        assert_eq!(update.gradients.len(), 3);
        assert_eq!(update.accuracy, 0.0);
    }

    #[test]
    fn test_gradient_averaging() {
        let mut update1 = GradientUpdate::new(
            crate::federation::specialist::SpecialistId::Visionary,
            vec![0.1, 0.2],
        );
        update1.accuracy = 0.9;

        let mut update2 = GradientUpdate::new(
            crate::federation::specialist::SpecialistId::Visionary,
            vec![0.2, 0.4],
        );
        update2.accuracy = 0.85;

        let avg = update1.average_with(&update2);
        assert_eq!(avg.gradients.len(), 2);
        assert!((avg.accuracy - 0.875).abs() < 0.01);
    }

    #[test]
    fn test_model_merger() {
        let merger = ModelMerger::new();
        assert_eq!(merger.merge_strategy, MergeStrategy::FederatedAverage);
    }

    #[test]
    fn test_federated_learning_engine() {
        let mut engine = FederatedLearningEngine::new();
        let updates = vec![GradientUpdate::new(
            crate::federation::specialist::SpecialistId::Visionary,
            vec![0.1, 0.2],
        )];

        let acc = engine.train_round(updates);
        assert!(acc >= 0.0 && acc <= 1.0);
        assert_eq!(engine.rounds_completed, 1);
    }
}
