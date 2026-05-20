use crate::data_ingestion::IngestibleData;
use crate::content_analyzer::ContentAnalysis;
use crate::capability_matcher::{CapabilityMatcher, CapabilityMatch, SkillTrainingExample};
use crate::event_loop::SkillExecutionEvent;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Distillation Engine: Transforms ingested data into XP-generating events
pub struct DistillationEngine {
    /// Capability matcher for routing
    matcher: CapabilityMatcher,
    /// Configuration
    config: DistillationConfig,
}

/// Configuration for distillation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillationConfig {
    /// Quality score threshold (0.0-1.0)
    pub quality_threshold: f32,
    /// Minimum confidence for capability matching
    pub match_confidence_threshold: f32,
    /// Maximum matches per data item
    pub max_matches: usize,
    /// Whether to create events immediately
    pub auto_create_events: bool,
    /// Base XP value per data item
    pub base_xp: u32,
}

impl Default for DistillationConfig {
    fn default() -> Self {
        Self {
            quality_threshold: 0.5,
            match_confidence_threshold: 0.6,
            max_matches: 3,
            auto_create_events: true,
            base_xp: 100,
        }
    }
}

/// Result of distillation process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillationResult {
    /// ID of ingested data
    pub data_id: String,
    /// Matches found
    pub matches: Vec<CapabilityMatch>,
    /// Training examples created
    pub training_examples: Vec<SkillTrainingExample>,
    /// Events generated
    pub events: Vec<SkillExecutionEvent>,
    /// Overall quality assessment
    pub quality_assessment: QualityAssessment,
    /// Distillation timestamp
    pub distilled_at: DateTime<Utc>,
}

/// Quality assessment of ingested data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAssessment {
    /// Overall quality score (0.0-1.0)
    pub overall_score: f32,
    /// Format quality (how well-structured)
    pub format_quality: f32,
    /// Semantic quality (clarity of content)
    pub semantic_quality: f32,
    /// Training value (usefulness for skill development)
    pub training_value: f32,
    /// Assessment details
    pub notes: Vec<String>,
}

impl DistillationEngine {
    /// Create a new distillation engine
    pub fn new(config: DistillationConfig) -> Self {
        Self {
            matcher: CapabilityMatcher::load_default_specialists(),
            config,
        }
    }

    /// Create with default configuration
    pub fn default_with_matcher(matcher: CapabilityMatcher) -> Self {
        Self {
            matcher,
            config: DistillationConfig::default(),
        }
    }

    /// Distill ingested data into training examples and events
    pub fn distill(&self, data: &IngestibleData, analysis: &ContentAnalysis) -> DistillationResult {
        // Find capability matches
        let matches = self.matcher.find_matches(data, analysis, self.config.max_matches);
        
        // Apply complexity scoring
        let scored_matches = self.matcher.apply_complexity_scoring(matches.clone(), analysis.complexity);

        // Filter by confidence threshold
        let filtered_matches: Vec<_> = scored_matches
            .iter()
            .filter(|m| m.confidence >= self.config.match_confidence_threshold)
            .cloned()
            .collect();

        // Create training examples
        let training_examples: Vec<_> = filtered_matches
            .iter()
            .map(|m| self.matcher.create_training_example(data, m))
            .collect();

        // Assess data quality
        let quality_assessment = self.assess_quality(data, analysis, &filtered_matches);

        // Generate events
        let events = if self.config.auto_create_events {
            self.generate_events(data, &training_examples, &quality_assessment)
        } else {
            Vec::new()
        };

        DistillationResult {
            data_id: data.id.clone(),
            matches: filtered_matches,
            training_examples,
            events,
            quality_assessment,
            distilled_at: Utc::now(),
        }
    }

    /// Assess data quality for training purposes
    fn assess_quality(
        &self,
        _data: &IngestibleData,
        analysis: &ContentAnalysis,
        matches: &[CapabilityMatch],
    ) -> QualityAssessment {
        let mut notes = Vec::new();

        // Format quality assessment
        let format_quality = if analysis.structure.fields.is_empty() {
            0.4 // Unstructured data
        } else {
            0.9 // Well-structured data
        };

        if format_quality < 0.5 {
            notes.push("Data has unclear structure".to_string());
        }

        // Semantic quality assessment
        let semantic_quality = if analysis.key_terms.is_empty() {
            0.3
        } else if analysis.domains.len() > 3 {
            0.9
        } else {
            0.7
        };

        if semantic_quality < 0.5 {
            notes.push("Semantic content is ambiguous".to_string());
        }

        // Training value assessment
        let training_value = if matches.is_empty() {
            0.2
        } else {
            let avg_confidence = matches.iter().map(|m| m.confidence).sum::<f32>() / matches.len() as f32;
            0.5 + (avg_confidence * 0.5) // Scales 0.5-1.0
        };

        let overall_score = (format_quality * 0.3 + semantic_quality * 0.3 + training_value * 0.4).min(1.0);

        // Add quality notes
        if overall_score > 0.8 {
            notes.push("Excellent training data".to_string());
        } else if overall_score > 0.6 {
            notes.push("Good training data".to_string());
        } else if overall_score > 0.4 {
            notes.push("Moderate training value".to_string());
        } else {
            notes.push("Limited training value".to_string());
        }

        QualityAssessment {
            overall_score,
            format_quality,
            semantic_quality,
            training_value,
            notes,
        }
    }

    /// Generate skill execution events from training examples
    fn generate_events(
        &self,
        _data: &IngestibleData,
        training_examples: &[SkillTrainingExample],
        quality: &QualityAssessment,
    ) -> Vec<SkillExecutionEvent> {
        training_examples
            .iter()
            .map(|example| {
                // Calculate XP
                let base_xp = self.config.base_xp;
                let quality_multiplier = quality.training_value;
                let difficulty_xp = (base_xp as f32 * example.difficulty_multiplier) as u32;
                let total_xp = (difficulty_xp as f32 * quality_multiplier) as u32;

                // Create event matching actual SkillExecutionEvent structure
                let mut event = SkillExecutionEvent::new(
                    example.specialist_id.clone(),
                    uuid::Uuid::new_v4().to_string(),
                    format!("{:?}", example.skill_type),
                    true, // Data ingestion is inherently successful
                    example.quality_score as f64,
                );
                event.difficulty_multiplier = example.difficulty_multiplier as f64;
                event.xp_awarded = total_xp;
                event
            })
            .collect()
    }
}

/// Data Crystallizer: Converts distillation results into events
pub struct DataCrystallizer;

impl DataCrystallizer {
    /// Crystallize a distillation result into a structured form
    pub fn crystallize(result: DistillationResult) -> CrystallizedData {
        CrystallizedData {
            data_id: result.data_id,
            specialist_assignments: result
                .training_examples
                .iter()
                .map(|ex| (ex.specialist_id.clone(), ex.skill_type.clone()))
                .collect(),
            xp_awards: result
                .events
                .iter()
                .map(|ev| (ev.specialist_id.clone(), ev.xp_awarded))
                .collect(),
            quality: result.quality_assessment.overall_score,
            domains_identified: result
                .matches
                .iter()
                .flat_map(|m| {
                    // Extract domain from reason
                    if m.reason.contains("database") {
                        vec!["database".to_string()]
                    } else if m.reason.contains("networking") {
                        vec!["networking".to_string()]
                    } else if m.reason.contains("security") {
                        vec!["security".to_string()]
                    } else {
                        vec![]
                    }
                })
                .collect(),
            crystallized_at: result.distilled_at,
        }
    }

    /// Batch crystallize multiple results
    pub fn batch_crystallize(results: Vec<DistillationResult>) -> Vec<CrystallizedData> {
        results.into_iter().map(Self::crystallize).collect()
    }
}

/// Crystallized data ready for event broadcasting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizedData {
    pub data_id: String,
    pub specialist_assignments: HashMap<String, crate::skill_system::SkillType>,
    pub xp_awards: HashMap<String, u32>,
    pub quality: f32,
    pub domains_identified: Vec<String>,
    pub crystallized_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_system::SkillType;

    #[test]
    fn test_distillation_engine_creation() {
        let engine = DistillationEngine::new(DistillationConfig::default());
        assert_eq!(engine.config.base_xp, 100);
    }

    #[test]
    fn test_quality_assessment() {
        let data = IngestibleData::from_payload("test data".to_string(), "text/plain".to_string());
        let mut analysis = ContentAnalysis {
            domains: HashMap::new(),
            key_terms: vec![("test".to_string(), 1)],
            structure: crate::content_analyzer::StructuralAnalysis::default(),
            complexity: 0.5,
        };
        
        analysis.domains.insert("database".to_string(), 0.8);

        let engine = DistillationEngine::new(DistillationConfig::default());
        let assessment = engine.assess_quality(&data, &analysis, &[]);

        assert!(assessment.overall_score > 0.0);
        assert!(assessment.overall_score <= 1.0);
    }

    #[test]
    fn test_event_generation() {
        let training_example = SkillTrainingExample {
            data_id: "test".to_string(),
            specialist_id: "ariel".to_string(),
            skill_type: SkillType::RAG,
            quality_score: 0.9,
            difficulty_multiplier: 1.5,
            description: "test".to_string(),
        };

        let quality = QualityAssessment {
            overall_score: 0.8,
            format_quality: 0.9,
            semantic_quality: 0.8,
            training_value: 0.8,
            notes: vec![],
        };

        let data = IngestibleData::from_payload("test".to_string(), "text/plain".to_string());
        let engine = DistillationEngine::new(DistillationConfig::default());
        let events = engine.generate_events(&data, &[training_example], &quality);

        assert!(!events.is_empty());
        assert!(events[0].xp_awarded > 0);
    }

    #[test]
    fn test_data_crystallizer() {
        let result = DistillationResult {
            data_id: "test_data".to_string(),
            matches: vec![],
            training_examples: vec![SkillTrainingExample {
                data_id: "test".to_string(),
                specialist_id: "ariel".to_string(),
                skill_type: SkillType::RAG,
                quality_score: 0.9,
                difficulty_multiplier: 1.5,
                description: "test".to_string(),
            }],
            events: vec![],
            quality_assessment: QualityAssessment {
                overall_score: 0.8,
                format_quality: 0.9,
                semantic_quality: 0.8,
                training_value: 0.8,
                notes: vec![],
            },
            distilled_at: Utc::now(),
        };

        let crystallized = DataCrystallizer::crystallize(result);
        assert_eq!(crystallized.data_id, "test_data");
        assert!(!crystallized.specialist_assignments.is_empty());
    }
}
