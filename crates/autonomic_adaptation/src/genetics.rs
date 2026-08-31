// Aaroneous Genetics Module
// Extraction, analysis, and management of specialist genetic profiles from GGUF models
// Based on weight analysis, behavioral profiling, and DAG/RAG pattern extraction

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A genetic locus - a single inherited trait with a value on spectrum [0.0, 1.0]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneticLocus {
    pub locus_id: String,
    pub category: GeneticCategory,
    pub value: f64,             // 0.0 to 1.0
    pub source: LociSource,     // Where this value came from
    pub interpretation: String, // Human-readable meaning
    pub confidence: f64,        // How confident in this measurement
}

impl GeneticLocus {
    pub fn new(id: String, category: GeneticCategory, value: f64, source: LociSource) -> Self {
        assert!(
            (0.0..=1.0).contains(&value),
            "Genetic value must be in [0.0, 1.0]"
        );
        Self {
            locus_id: id,
            category,
            value,
            source,
            interpretation: String::new(),
            confidence: 1.0,
        }
    }

    pub fn with_interpretation(mut self, interpretation: String) -> Self {
        self.interpretation = interpretation;
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = (confidence).clamp(0.0, 1.0);
        self
    }
}

/// Categories of genetic loci
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GeneticCategory {
    AttentionGenetics,      // Multi-head attention patterns
    LayerGenetics,          // Per-layer characteristics
    EmbeddingGenetics,      // Token embedding space properties
    BiasGenetics,           // Systematic biases
    DAGGenetics,            // Task decomposition patterns
    RAGGenetics,            // Information retrieval preferences
    PersonalityGenetics,    // Cognitive style and communication
    SpecializationGenetics, // Domain expertise
}

/// Source of a genetic locus value
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LociSource {
    WeightAnalysis,           // From model weight matrices
    AttentionPatternAnalysis, // From attention head behavior
    BehavioralProfiling,      // From test suite runs
    DAGAnalysis,              // From decomposition studies
    RAGAnalysis,              // From retrieval studies
    Inferred,                 // Computed from other metrics
}

/// Epigenetic markers that regulate gene expression
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpigeneticState {
    /// 0.0 = gene fully expressed, 1.0 = gene silenced
    pub methylation: f64,
    /// 0.0 = locked (immutable), 1.0 = open (trainable)
    pub chromatin_accessibility: f64,
    /// -1.0 = suppressed, 0.0 = baseline, 1.0 = amplified
    pub histone_modification: f64,
}

impl Default for EpigeneticState {
    fn default() -> Self {
        Self {
            methylation: 0.5,
            chromatin_accessibility: 0.5,
            histone_modification: 0.0,
        }
    }
}

/// Complete genetic profile of a specialist
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecialistGenome {
    pub specialist_id: String,
    pub specialist_name: String,
    pub base_model: String, // Source GGUF model name
    pub genetic_loci: Vec<GeneticLocus>,
    pub epigenetic_state: EpigeneticState,
    pub extracted_at: DateTime<Utc>,
    pub extraction_version: u32,
    pub trait_expression_profile: HashMap<String, f64>,
    pub genetic_distance_to_base: f64,
    pub specialization_score: f64,
}

pub type ParameterLocus = GeneticLocus;
pub type AgentConfigProfile = SpecialistGenome;
pub type ParameterGenome = SpecialistGenome;

impl SpecialistGenome {
    pub fn new(specialist_id: String, specialist_name: String, base_model: String) -> Self {
        Self {
            specialist_id,
            specialist_name,
            base_model,
            genetic_loci: Vec::new(),
            epigenetic_state: EpigeneticState::default(),
            extracted_at: Utc::now(),
            extraction_version: 1,
            trait_expression_profile: HashMap::new(),
            genetic_distance_to_base: 0.0,
            specialization_score: 0.0,
        }
    }

    /// Add a genetic locus to this genome
    pub fn add_locus(&mut self, locus: GeneticLocus) {
        self.genetic_loci.push(locus);
    }

    /// Get average genetic value for a category
    pub fn category_average(&self, category: GeneticCategory) -> f64 {
        let loci: Vec<_> = self
            .genetic_loci
            .iter()
            .filter(|l| l.category == category)
            .collect();

        if loci.is_empty() {
            return 0.5; // Default middle value
        }

        let sum: f64 = loci.iter().map(|l| l.value).sum();
        sum / loci.len() as f64
    }

    /// Get all loci in a category
    pub fn loci_in_category(&self, category: GeneticCategory) -> Vec<&GeneticLocus> {
        self.genetic_loci
            .iter()
            .filter(|l| l.category == category)
            .collect()
    }

    /// Calculate actual expressed trait values considering epigenetics
    pub fn expressed_trait_value(&self, locus: &GeneticLocus) -> f64 {
        let base_value = locus.value;

        // Apply epigenetic modulation
        let methylation_effect = 1.0 - locus.value.abs() * self.epigenetic_state.methylation;
        let modification_effect = 1.0 + self.epigenetic_state.histone_modification;
        let accessibility_factor = self.epigenetic_state.chromatin_accessibility;

        // Combined epigenetic expression
        (base_value * methylation_effect * modification_effect * accessibility_factor)
            .clamp(0.0, 1.0)
    }

    /// Count total genetic loci
    pub fn locus_count(&self) -> usize {
        self.genetic_loci.len()
    }

    /// Calculate genetic diversity (entropy of locus values)
    pub fn genetic_diversity(&self) -> f64 {
        if self.genetic_loci.is_empty() {
            return 0.0;
        }

        let mut entropy = 0.0;
        for locus in &self.genetic_loci {
            let p = locus.value;
            if p > 0.0 && p < 1.0 {
                entropy -= p * p.log2() + (1.0 - p) * (1.0 - p).log2();
            }
        }

        entropy / self.genetic_loci.len() as f64
    }
}

/// Genetic relationship between two specialists (for breeding)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneticRelationship {
    pub parent_1_id: String,
    pub parent_2_id: String,
    pub genetic_distance: f64, // 0.0 = identical, 1.0 = completely different
    pub shared_loci_count: usize,
    pub divergent_loci_count: usize,
    pub breeding_compatibility_score: f64, // 0.0 to 1.0
}

impl GeneticRelationship {
    /// Calculate genetic distance between two genomes (simplified Euclidean)
    pub fn calculate_distance(genome1: &SpecialistGenome, genome2: &SpecialistGenome) -> f64 {
        let mut sum_sq = 0.0;
        let mut count = 0;

        for locus1 in &genome1.genetic_loci {
            if let Some(locus2) = genome2
                .genetic_loci
                .iter()
                .find(|l| l.locus_id == locus1.locus_id)
            {
                let diff = locus1.value - locus2.value;
                sum_sq += diff * diff;
                count += 1;
            }
        }

        if count == 0 {
            return 1.0; // Completely different
        }

        (sum_sq / count as f64).sqrt()
    }
}

/// Breeding operation to create specialist variants
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BreedingOperation {
    pub operation_id: String,
    pub parent_1_id: String,
    pub parent_2_id: String,
    pub breeding_type: BreedingType,
    pub offspring_id: String,
    pub created_at: DateTime<Utc>,
    pub success_score: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum BreedingType {
    SimpleUniformCrossover,  // 50/50 inheritance
    WeightedBlending,        // Weighted combination
    EpigeneticRecombination, // Different expression of same genetics
    TargetedLocusSwap,       // Replace specific genetic regions
}

impl BreedingOperation {
    /// Perform simple uniform crossover
    pub fn simple_crossover(
        parent1: &SpecialistGenome,
        parent2: &SpecialistGenome,
        offspring_id: String,
    ) -> SpecialistGenome {
        let mut offspring = SpecialistGenome::new(
            offspring_id,
            format!("{}_x_{}", parent1.specialist_name, parent2.specialist_name),
            format!(
                "hybrid_of_{}_and_{}",
                parent1.base_model, parent2.base_model
            ),
        );

        // For each locus, randomly select from parent with 50% probability
        for (i, parent1_locus) in parent1.genetic_loci.iter().enumerate() {
            let parent2_locus = parent2
                .genetic_loci
                .iter()
                .find(|l| l.locus_id == parent1_locus.locus_id);

            let selected_locus = if i % 2 == 0 {
                parent1_locus.clone()
            } else if let Some(l2) = parent2_locus {
                l2.clone()
            } else {
                parent1_locus.clone()
            };

            offspring.add_locus(selected_locus);
        }

        offspring.epigenetic_state.methylation =
            (parent1.epigenetic_state.methylation + parent2.epigenetic_state.methylation) / 2.0;
        offspring.genetic_distance_to_base =
            (parent1.genetic_distance_to_base + parent2.genetic_distance_to_base) / 2.0;

        offspring
    }

    /// Perform weighted blending of two genomes
    pub fn weighted_blend(
        parent1: &SpecialistGenome,
        parent2: &SpecialistGenome,
        weight1: f64,
        weight2: f64,
        offspring_id: String,
    ) -> SpecialistGenome {
        assert!(
            (weight1 + weight2 - 1.0).abs() < 0.01,
            "Weights must sum to 1.0"
        );

        let mut offspring = SpecialistGenome::new(
            offspring_id,
            format!(
                "blend({:.0}% {} + {:.0}% {})",
                weight1 * 100.0,
                parent1.specialist_name,
                weight2 * 100.0,
                parent2.specialist_name
            ),
            format!(
                "hybrid_of_{}_and_{}",
                parent1.base_model, parent2.base_model
            ),
        );

        for parent1_locus in &parent1.genetic_loci {
            if let Some(parent2_locus) = parent2
                .genetic_loci
                .iter()
                .find(|l| l.locus_id == parent1_locus.locus_id)
            {
                let blended_value = parent1_locus.value * weight1 + parent2_locus.value * weight2;

                let mut blended_locus = GeneticLocus::new(
                    parent1_locus.locus_id.clone(),
                    parent1_locus.category,
                    blended_value,
                    LociSource::Inferred,
                );
                blended_locus.confidence =
                    (parent1_locus.confidence + parent2_locus.confidence) / 2.0;

                offspring.add_locus(blended_locus);
            }
        }

        offspring
    }
}

/// Genetic analysis and comparison utilities
pub struct GeneticAnalyzer;

impl GeneticAnalyzer {
    /// Calculate genetic distance between two specialists
    pub fn distance(genome1: &SpecialistGenome, genome2: &SpecialistGenome) -> f64 {
        GeneticRelationship::calculate_distance(genome1, genome2)
    }

    /// Find most genetically similar specialist in a population
    pub fn find_closest_relative<'a>(
        query_genome: &SpecialistGenome,
        population: &'a [SpecialistGenome],
    ) -> Option<&'a SpecialistGenome> {
        population.iter().min_by(|a, b| {
            let dist_a = Self::distance(query_genome, a);
            let dist_b = Self::distance(query_genome, b);
            dist_a.partial_cmp(&dist_b).unwrap()
        })
    }

    /// Measure genetic diversity in a population
    pub fn population_diversity(population: &[SpecialistGenome]) -> f64 {
        if population.len() < 2 {
            return 0.0;
        }

        let mut total_distance = 0.0;
        let mut comparisons = 0;

        for i in 0..population.len() {
            for j in (i + 1)..population.len() {
                total_distance += Self::distance(&population[i], &population[j]);
                comparisons += 1;
            }
        }

        if comparisons == 0 {
            0.0
        } else {
            total_distance / comparisons as f64
        }
    }

    /// Find optimal breeding pairs for desired traits
    pub fn find_breeding_candidates<'a>(
        population: &'a [SpecialistGenome],
        desired_traits: &HashMap<String, f64>,
    ) -> Vec<(&'a SpecialistGenome, &'a SpecialistGenome)> {
        let mut candidates = Vec::new();

        for i in 0..population.len() {
            for j in (i + 1)..population.len() {
                let score_i = Self::trait_match_score(&population[i], desired_traits);
                let score_j = Self::trait_match_score(&population[j], desired_traits);

                if score_i + score_j > 1.0 {
                    // Good complementary match
                    candidates.push((&population[i], &population[j]));
                }
            }
        }

        candidates.sort_by(|a, b| {
            let score_a = Self::trait_match_score(a.0, desired_traits)
                + Self::trait_match_score(a.1, desired_traits);
            let score_b = Self::trait_match_score(b.0, desired_traits)
                + Self::trait_match_score(b.1, desired_traits);
            score_b.partial_cmp(&score_a).unwrap()
        });

        candidates
    }

    /// Calculate how well a specialist matches desired traits
    fn trait_match_score(genome: &SpecialistGenome, desired_traits: &HashMap<String, f64>) -> f64 {
        let mut total_score = 0.0;
        let mut count = 0;

        for (trait_name, desired_value) in desired_traits {
            if let Some(actual_value) = genome.trait_expression_profile.get(trait_name) {
                let diff = (actual_value - desired_value).abs();
                total_score += 1.0 - diff; // Perfect match = 1.0, complete mismatch = 0.0
                count += 1;
            }
        }

        if count == 0 {
            0.0
        } else {
            total_score / count as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genetic_locus_creation() {
        let locus = GeneticLocus::new(
            "ATT_1".to_string(),
            GeneticCategory::AttentionGenetics,
            0.75,
            LociSource::WeightAnalysis,
        );

        assert_eq!(locus.locus_id, "ATT_1");
        assert_eq!(locus.value, 0.75);
        assert_eq!(locus.confidence, 1.0);
    }

    #[test]
    #[should_panic]
    fn test_genetic_value_bounds() {
        let _locus = GeneticLocus::new(
            "BAD_LOCUS".to_string(),
            GeneticCategory::LayerGenetics,
            1.5, // Out of bounds
            LociSource::BehavioralProfiling,
        );
    }

    #[test]
    fn test_epigenetic_state() {
        let state = EpigeneticState::default();
        assert_eq!(state.methylation, 0.5);
        assert_eq!(state.chromatin_accessibility, 0.5);
        assert_eq!(state.histone_modification, 0.0);
    }

    #[test]
    fn test_genome_creation() {
        let mut genome = SpecialistGenome::new(
            "orchestrator_1".to_string(),
            "Orchestrator".to_string(),
            "llama2-70b".to_string(),
        );

        let locus = GeneticLocus::new(
            "STRAT_1".to_string(),
            GeneticCategory::DAGGenetics,
            0.87,
            LociSource::DAGAnalysis,
        );

        genome.add_locus(locus);
        assert_eq!(genome.locus_count(), 1);
    }

    #[test]
    fn test_simple_crossover() {
        let mut parent1 = SpecialistGenome::new(
            "p1".to_string(),
            "Parent1".to_string(),
            "model1".to_string(),
        );
        let mut parent2 = SpecialistGenome::new(
            "p2".to_string(),
            "Parent2".to_string(),
            "model2".to_string(),
        );

        let locus1 = GeneticLocus::new(
            "L1".to_string(),
            GeneticCategory::AttentionGenetics,
            0.9,
            LociSource::WeightAnalysis,
        );
        let locus2 = GeneticLocus::new(
            "L1".to_string(),
            GeneticCategory::AttentionGenetics,
            0.3,
            LociSource::WeightAnalysis,
        );

        parent1.add_locus(locus1);
        parent2.add_locus(locus2);

        let offspring =
            BreedingOperation::simple_crossover(&parent1, &parent2, "offspring".to_string());

        assert_eq!(offspring.locus_count(), 1);
        let offspring_value = offspring.genetic_loci[0].value;
        assert!(offspring_value == 0.9 || offspring_value == 0.3);
    }

    #[test]
    fn test_genetic_distance() {
        let mut genome1 = SpecialistGenome::new(
            "g1".to_string(),
            "Specialist1".to_string(),
            "model".to_string(),
        );
        let mut genome2 = SpecialistGenome::new(
            "g2".to_string(),
            "Specialist2".to_string(),
            "model".to_string(),
        );

        let locus1 = GeneticLocus::new(
            "L1".to_string(),
            GeneticCategory::AttentionGenetics,
            0.5,
            LociSource::WeightAnalysis,
        );
        let locus2 = GeneticLocus::new(
            "L1".to_string(),
            GeneticCategory::AttentionGenetics,
            0.8,
            LociSource::WeightAnalysis,
        );

        genome1.add_locus(locus1);
        genome2.add_locus(locus2);

        let distance = GeneticAnalyzer::distance(&genome1, &genome2);
        assert!((distance - 0.3).abs() < 0.01);
    }
}
