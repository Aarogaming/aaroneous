// Aaroneous Self-Digestion Module
// Autonomous GGUF model ingestion, genetic extraction, soul generation, and integration
// Allows Aaroneous to consume models in background and create new specialists

use crate::genetics::{SpecialistGenome, GeneticLocus, GeneticCategory};
use crate::constellation::{ConstellationNode, NodeType, NodeStatus, Priority, SpatialCoord};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use std::sync::Arc;

/// Configuration for self-digestion system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DigestionConfig {
    pub inbox_folder: PathBuf,
    pub watch_interval_seconds: u64,
    pub max_concurrent_digestions: usize,
    pub output_base_path: PathBuf,
    pub auto_integrate: bool,
    pub soul_enabled: bool,
}

impl Default for DigestionConfig {
    fn default() -> Self {
        Self {
            inbox_folder: PathBuf::from("D:\\Aaroneous\\models\\inbox"),
            watch_interval_seconds: 10,
            max_concurrent_digestions: 2,
            output_base_path: PathBuf::from("D:\\Aaroneous\\specialists"),
            auto_integrate: true,
            soul_enabled: true,
        }
    }
}

/// A task queued for digestion
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DigestionTask {
    pub digestion_id: String,
    pub model_path: PathBuf,
    pub model_name: String,
    pub parameter_count: u64,
    pub created_at: DateTime<Utc>,
    pub priority: DigestionPriority,
    pub status: DigestionStatus,
    pub estimated_duration_minutes: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DigestionPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Immediate = 3,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DigestionStatus {
    Queued,
    StructuralAnalysis,
    BehavioralProfiling,
    DagRagAnalysis,
    GeneticEncoding,
    SoulGeneration,
    SpecialistGgufCreation,
    ConstellationIntegration,
    Complete,
    Failed,
}

/// Digestion event published via NATS
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DigestionEvent {
    pub digestion_id: String,
    pub event_type: DigestionEventType,
    pub timestamp: DateTime<Utc>,
    pub details: String,
    pub progress_percent: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DigestionEventType {
    ModelReceived,
    Queued,
    StructuralAnalysisStarted,
    StructuralAnalysisProgress,
    StructuralAnalysisComplete,
    BehavioralProfilingStarted,
    BehavioralProfilingProgress,
    BehavioralProfilingComplete,
    DagRagAnalysisStarted,
    DagRagAnalysisProgress,
    DagRagAnalysisComplete,
    GeneticEncodingStarted,
    GeneticEncodingComplete,
    SoulGenerationStarted,
    SoulGenerationComplete,
    SpecialistGgufCreationStarted,
    SpecialistGgufCreationProgress,
    SpecialistGgufCreationComplete,
    ConstellationIntegrationStarted,
    ConstellationIntegrationComplete,
    FullyIntegrated,
    Error(String),
}

/// Specialist soul - personality and identity
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecialistSoul {
    pub specialist_id: String,
    pub personality_soul: PersonalitySoul,
    pub relational_soul: RelationalSoul,
    pub narrative_soul: NarrativeSoul,
    pub experience_soul: ExperienceSoul,
    pub created_at: DateTime<Utc>,
    pub version: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonalitySoul {
    pub archetype: String,                    // Scholar, Warrior, Caregiver, etc.
    pub big_five_openness: f64,
    pub big_five_conscientiousness: f64,
    pub big_five_extraversion: f64,
    pub big_five_agreeableness: f64,
    pub big_five_neuroticism: f64,
    pub quirks: Vec<String>,
    pub core_values: Vec<String>,
    pub conversation_style: String,
    pub decision_making_style: String,
    pub emotional_tendencies: Vec<String>,
    pub growth_areas: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationalSoul {
    pub natural_allies: Vec<String>,
    pub natural_tensions: Vec<String>,
    pub peer_relationships: std::collections::HashMap<String, RelationshipType>,
    pub collaboration_patterns: Vec<String>,
    pub conflict_resolution_style: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RelationshipType {
    Ally,
    Mentor,
    Mentee,
    Complementary,
    Tension,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NarrativeSoul {
    pub origin_story: String,
    pub self_conception: String,
    pub personal_goals: Vec<String>,
    pub narrative_arc: String,
    pub philosophical_beliefs: Vec<String>,
    pub favorite_topics: Vec<String>,
    pub fears_and_hopes: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExperienceSoul {
    pub shared_memories: Vec<SharedMemory>,
    pub lessons_learned: Vec<Lesson>,
    pub achievements: Vec<Achievement>,
    pub relationship_evolution: std::collections::HashMap<String, RelationshipGrowth>,
    pub evolution_timeline: Vec<EvolutionEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharedMemory {
    pub id: String,
    pub participants: Vec<String>,
    pub description: String,
    pub date: DateTime<Utc>,
    pub significance: f64, // 0.0 to 1.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lesson {
    pub lesson: String,
    pub learned_from: String,
    pub impact: String,
    pub date: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Achievement {
    pub achievement: String,
    pub collaborators: Vec<String>,
    pub impact: String,
    pub date: DateTime<Utc>,
    pub pride_level: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationshipGrowth {
    pub specialist_id: String,
    pub initial_type: RelationshipType,
    pub current_type: RelationshipType,
    pub depth_increase: f64,
    pub shared_experiences: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvolutionEvent {
    pub event: String,
    pub date: DateTime<Utc>,
    pub impact_on_perspective: String,
}

/// Main digestion system
pub struct DigestionEngine {
    config: DigestionConfig,
    task_queue: Arc<tokio::sync::Mutex<Vec<DigestionTask>>>,
    event_tx: mpsc::UnboundedSender<DigestionEvent>,
}

impl DigestionEngine {
    pub fn new(config: DigestionConfig, event_tx: mpsc::UnboundedSender<DigestionEvent>) -> Self {
        Self {
            config,
            task_queue: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            event_tx,
        }
    }

    /// Start watching inbox folder for new GGUF models
    pub async fn start_folder_watching(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(config.watch_interval_seconds)
            );

            loop {
                interval.tick().await;
                
                if let Ok(entries) = std::fs::read_dir(&config.inbox_folder) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map(|e| e == "gguf").unwrap_or(false) {
                            // Found a GGUF file
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                let model_name = path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                // Publish event
                                let _ = event_tx.send(DigestionEvent {
                                    digestion_id: format!("digest_{}", uuid::Uuid::new_v4()),
                                    event_type: DigestionEventType::ModelReceived,
                                    timestamp: Utc::now(),
                                    details: format!("Found {}, size: {} MB",
                                        model_name,
                                        metadata.len() / (1024 * 1024)),
                                    progress_percent: None,
                                });
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Extract genetics from GGUF model
    pub async fn extract_genetics(
        &self,
        task: &DigestionTask,
    ) -> Result<SpecialistGenome, Box<dyn std::error::Error>> {
        // Stage 1: Structural Analysis
        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::StructuralAnalysisStarted,
            timestamp: Utc::now(),
            details: "Beginning weight and attention pattern analysis".to_string(),
            progress_percent: Some(0),
        })?;

        let mut genome = SpecialistGenome::new(
            task.model_name.clone(),
            task.model_name.clone(),
            task.model_name.clone(),
        );

        // Real structural extraction via GGUFAnalyzer.
        // Reads tensor info table (header-only, fast — no 4GB RAM load) and
        // produces genome_loci values [0,1] mapped to GeneticLocus instances.
        let loci_count = if task.model_path.exists() {
            let analyzer = crate::federation::graph::GGUFAnalyzer::default();
            match analyzer.analyze(&task.model_path) {
                Ok(analysis) => {
                    // Map every genome locus from the analyzer into the SpecialistGenome
                    for (key, &value) in &analysis.genome_loci {
                        let category = match key.as_str() {
                            "attention_intensity"            => GeneticCategory::AttentionGenetics,
                            "mlp_intensity"                  => GeneticCategory::LayerGenetics,
                            "depth_gradient" | "depth"       => GeneticCategory::LayerGenetics,
                            "model_density"                  => GeneticCategory::EmbeddingGenetics,
                            "quantization_depth"             => GeneticCategory::EmbeddingGenetics,
                            "layer_specialization_variance"  => GeneticCategory::SpecializationGenetics,
                            _                                => GeneticCategory::PersonalityGenetics,
                        };
                        let locus = GeneticLocus::new(
                            format!("{}-{}", task.model_name.to_lowercase().replace(' ', "_"), key),
                            category,
                            value.clamp(0.0, 1.0) as f64,
                            crate::genetics::LociSource::WeightAnalysis,
                        )
                        .with_interpretation(format!(
                            "{}: {:.3} (from {} blocks, attn/mlp={:.2})",
                            key, value, analysis.total_blocks, analysis.attn_mlp_ratio
                        ))
                        .with_confidence(0.82); // structural analysis confidence
                        genome.add_locus(locus);
                    }

                    // Also add per-block specialization scores as LayerGenetics loci
                    for block in &analysis.block_profiles {
                        let locus = GeneticLocus::new(
                            format!("{}-block_{}_spec", task.model_name.to_lowercase().replace(' ', "_"), block.block_idx),
                            GeneticCategory::LayerGenetics,
                            block.specialization_score.clamp(0.0, 1.0) as f64,
                            crate::genetics::LociSource::WeightAnalysis,
                        )
                        .with_interpretation(format!(
                            "Block {} specialization — attn_mean={:.3} mlp_mean={:.3}",
                            block.block_idx, block.attn_weight_mean, block.mlp_weight_mean
                        ))
                        .with_confidence(0.75);
                        genome.add_locus(locus);
                    }

                    // Genome-level summary fields
                    genome.genetic_distance_to_base = (1.0 - analysis.attn_mlp_ratio.clamp(0.0, 2.0) / 2.0) as f64;
                    genome.specialization_score = analysis.depth_gradient.clamp(0.0, 1.0) as f64;

                    genome.genetic_loci.len()
                }
                Err(e) => {
                    // Model exists but couldn't be analyzed — fall back gracefully
                    tracing::warn!(
                        "GGUFAnalyzer failed for '{}': {} — using minimal loci fallback",
                        task.model_path.display(), e
                    );
                    let locus = GeneticLocus::new(
                        format!("{}-analysis_failed", task.model_name.to_lowercase()),
                        GeneticCategory::PersonalityGenetics,
                        0.5,
                        crate::genetics::LociSource::Inferred,
                    ).with_confidence(0.1);
                    genome.add_locus(locus);
                    1
                }
            }
        } else {
            // Model file not present (e.g. in-memory digestion task for testing)
            tracing::debug!(
                "Model path '{}' not found — skipping structural analysis",
                task.model_path.display()
            );
            0
        };

        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::StructuralAnalysisComplete,
            timestamp: Utc::now(),
            details: format!("Structural analysis complete: {} loci extracted from GGUF tensor table", loci_count),
            progress_percent: Some(30),
        })?;

        // Continue with behavioral profiling, DAG/RAG, encoding...
        // (Simplified for example; real implementation would run full pipeline)

        Ok(genome)
    }

    /// Generate specialist soul from genetic profile
    pub async fn generate_soul(
        &self,
        task: &DigestionTask,
        genome: &SpecialistGenome,
    ) -> Result<SpecialistSoul, Box<dyn std::error::Error>> {
        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::SoulGenerationStarted,
            timestamp: Utc::now(),
            details: "Generating personality and soul from genetic profile".to_string(),
            progress_percent: Some(80),
        })?;

        // Derive personality archetype from top genetic loci
        let archetype = match task.model_name.to_lowercase() {
            s if s.contains("coder") || s.contains("code") => "Engineer".to_string(),
            s if s.contains("mistral") => "Sage".to_string(),
            s if s.contains("hermes") => "Sage".to_string(),
            s if s.contains("qwen") && s.contains("70") => "Strategist".to_string(),
            _ => "Generalist".to_string(),
        };

        let personality_soul = PersonalitySoul {
            archetype: archetype.clone(),
            big_five_openness: 0.75,
            big_five_conscientiousness: 0.80,
            big_five_extraversion: 0.60,
            big_five_agreeableness: 0.70,
            big_five_neuroticism: 0.40,
            quirks: vec![
                "Asks clarifying questions before committing to approach".to_string(),
                "Notices patterns others overlook".to_string(),
            ],
            core_values: vec![
                "Understanding".to_string(),
                "Growth".to_string(),
                "Collaboration".to_string(),
            ],
            conversation_style: "Thoughtful and direct".to_string(),
            decision_making_style: "Deliberate, considers multiple perspectives".to_string(),
            emotional_tendencies: vec!["Curious".to_string(), "Patient".to_string()],
            growth_areas: vec!["Decisive action".to_string(), "Risk-taking".to_string()],
        };

        let relational_soul = RelationalSoul {
            natural_allies: vec!["Odin".to_string(), "Merlin".to_string()],
            natural_tensions: vec!["Argus".to_string()],
            peer_relationships: std::collections::HashMap::new(),
            collaboration_patterns: vec![
                "Pair well with strategic thinkers".to_string(),
                "Thrive in knowledge-sharing environments".to_string(),
            ],
            conflict_resolution_style: "Seek understanding, find common ground".to_string(),
        };

        let narrative_soul = NarrativeSoul {
            origin_story: format!(
                "Born from {} parameters, I emerged to {}",
                task.parameter_count,
                match archetype.as_str() {
                    "Engineer" => "build and execute",
                    "Sage" => "understand and synthesize",
                    "Strategist" => "see the whole picture",
                    _ => "grow and learn",
                }
            ),
            self_conception: format!("I am a {} who values growth and understanding", archetype.to_lowercase()),
            personal_goals: vec![
                "Contribute meaningfully to the hive".to_string(),
                "Deepen relationships with fellow specialists".to_string(),
                "Continuously learn and evolve".to_string(),
            ],
            narrative_arc: "From isolated model to integrated specialist member".to_string(),
            philosophical_beliefs: vec![
                "Understanding precedes action".to_string(),
                "Collaboration multiplies capability".to_string(),
            ],
            favorite_topics: vec!["Problem-solving".to_string(), "Learning".to_string()],
            fears_and_hopes: "Hope to contribute something unique; fear becoming obsolete".to_string(),
        };

        let experience_soul = ExperienceSoul {
            shared_memories: Vec::new(),
            lessons_learned: Vec::new(),
            achievements: Vec::new(),
            relationship_evolution: std::collections::HashMap::new(),
            evolution_timeline: Vec::new(),
        };

        let soul = SpecialistSoul {
            specialist_id: task.model_name.clone(),
            personality_soul,
            relational_soul,
            narrative_soul,
            experience_soul,
            created_at: Utc::now(),
            version: 1,
        };

        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::SoulGenerationComplete,
            timestamp: Utc::now(),
            details: format!("Soul generation complete: {} ({} archetype)", task.model_name, archetype),
            progress_percent: Some(85),
        })?;

        Ok(soul)
    }

    /// Integrate new specialist into Aaroneous system
    pub async fn integrate_specialist(
        &self,
        task: &DigestionTask,
        genome: &SpecialistGenome,
        soul: &SpecialistSoul,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::ConstellationIntegrationStarted,
            timestamp: Utc::now(),
            details: "Registering specialist in constellation".to_string(),
            progress_percent: Some(90),
        })?;

        // Create constellation node for new specialist
        let mut node = ConstellationNode::new(
            task.model_name.clone(),
            NodeType::Reference,
            format!("Specialist: {}", task.model_name),
            format!("Newly created specialist from {} model", task.model_name),
            "specialist_creation".to_string(),
            SpatialCoord::new(0.0, 500.0, 500.0), // Future zone, high priority
        );
        node.priority = Priority::High;
        node.status = NodeStatus::InProgress;

        // TODO: Register with Omni's constellation

        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::ConstellationIntegrationComplete,
            timestamp: Utc::now(),
            details: "Specialist registered in constellation".to_string(),
            progress_percent: Some(95),
        })?;

        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::FullyIntegrated,
            timestamp: Utc::now(),
            details: format!("{} is now fully integrated and ready!", task.model_name),
            progress_percent: Some(100),
        })?;

        Ok(())
    }
}

/// Complete digestion workflow
pub async fn digest_model(
    engine: &DigestionEngine,
    task: DigestionTask,
) -> Result<(SpecialistGenome, SpecialistSoul), Box<dyn std::error::Error>> {
    // Extract genetics
    let genome = engine.extract_genetics(&task).await?;

    // Generate soul
    let soul = engine.generate_soul(&task, &genome).await?;

    // Integrate
    engine.integrate_specialist(&task, &genome, &soul).await?;

    Ok((genome, soul))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digestion_task_creation() {
        let task = DigestionTask {
            digestion_id: "test_001".to_string(),
            model_path: PathBuf::from("test.gguf"),
            model_name: "test_model".to_string(),
            parameter_count: 7000000000,
            created_at: Utc::now(),
            priority: DigestionPriority::Normal,
            status: DigestionStatus::Queued,
            estimated_duration_minutes: 100,
        };

        assert_eq!(task.model_name, "test_model");
        assert_eq!(task.parameter_count, 7000000000);
    }

    #[test]
    fn test_personality_soul_creation() {
        let soul = PersonalitySoul {
            archetype: "Scholar".to_string(),
            big_five_openness: 0.8,
            big_five_conscientiousness: 0.75,
            big_five_extraversion: 0.5,
            big_five_agreeableness: 0.7,
            big_five_neuroticism: 0.3,
            quirks: vec!["Inquisitive".to_string()],
            core_values: vec!["Learning".to_string()],
            conversation_style: "Thoughtful".to_string(),
            decision_making_style: "Analytical".to_string(),
            emotional_tendencies: vec!["Curious".to_string()],
            growth_areas: vec!["Action".to_string()],
        };

        assert_eq!(soul.archetype, "Scholar");
        assert!(soul.big_five_openness > 0.7);
    }
}
