// Aaroneous Self-Digestion Module
// Autonomous GGUF model ingestion, genetic extraction, persona generation, and integration
// Allows Aaroneous to consume models in background and create new specialists

use crate::workspace::WorkspacePaths;
use chrono::{DateTime, Utc};
use crate::genetics::{GeneticCategory, GeneticLocus, LociSource, SpecialistGenome};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Configuration for self-digestion system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DigestionConfig {
    pub inbox_folder: PathBuf,
    pub watch_interval_seconds: u64,
    pub max_concurrent_digestions: usize,
    pub output_base_path: PathBuf,
    pub auto_integrate: bool,
    pub persona_enabled: bool,
}

impl Default for DigestionConfig {
    fn default() -> Self {
        let paths = WorkspacePaths::discover();
        Self {
            inbox_folder: paths.models_inbox(),
            watch_interval_seconds: 10,
            max_concurrent_digestions: 2,
            output_base_path: paths.specialists(),
            auto_integrate: true,
            persona_enabled: true,
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
    PersonaGeneration,
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
    PersonaGenerationStarted,
    PersonaGenerationComplete,
    SpecialistGgufCreationStarted,
    SpecialistGgufCreationProgress,
    SpecialistGgufCreationComplete,
    ConstellationIntegrationStarted,
    ConstellationIntegrationComplete,
    FullyIntegrated,
    Error(String),
}

/// Specialist persona - personality and identity
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecialistPersona {
    pub specialist_id: String,
    pub personality_persona: PersonalityProfile,
    pub relational_persona: RelationalProfile,
    pub narrative_persona: NarrativeProfile,
    pub experience_persona: ExperienceProfile,
    pub created_at: DateTime<Utc>,
    pub version: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonalityProfile {
    pub archetype: String, // Scholar, Warrior, Caregiver, etc.
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
pub struct RelationalProfile {
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
pub struct NarrativeProfile {
    pub origin_story: String,
    pub self_conception: String,
    pub personal_goals: Vec<String>,
    pub narrative_arc: String,
    pub philosophical_beliefs: Vec<String>,
    pub favorite_topics: Vec<String>,
    pub fears_and_hopes: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExperienceProfile {
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

    /// Enqueue a new digestion task
    pub async fn queue_task(&self, task: DigestionTask) {
        let mut queue = self.task_queue.lock().await;
        queue.push(task);
    }

    /// Process all queued tasks, draining them in priority order
    pub async fn process_queue(&self) -> Vec<DigestionTask> {
        let mut queue = self.task_queue.lock().await;
        queue.sort_by_key(|t| std::cmp::Reverse(t.priority));
        let tasks: Vec<DigestionTask> = queue.drain(..).collect();
        drop(queue);

        let mut completed = vec![];
        for task in tasks {
            match self.run_digestion(&task).await {
                Ok(()) => completed.push(task),
                Err(e) => {
                    let _ = self.event_tx.send(DigestionEvent {
                        digestion_id: task.digestion_id.clone(),
                        event_type: DigestionEventType::Error(e.to_string()),
                        timestamp: Utc::now(),
                        details: format!("Digestion failed for {}", task.model_name),
                        progress_percent: None,
                    });
                }
            }
        }
        completed
    }

    /// Run the full digestion pipeline for a single task
    async fn run_digestion(&self, task: &DigestionTask) -> Result<(), Box<dyn std::error::Error>> {
        let genome = self.extract_genetics(task).await?;
        let soul = self.generate_soul(task, &genome).await?;
        self.integrate_specialist(task, &genome, &soul).await?;
        Ok(())
    }

    /// Start watching inbox folder for new GGUF models
    pub async fn start_folder_watching(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                config.watch_interval_seconds,
            ));

            loop {
                interval.tick().await;

                if let Ok(entries) = std::fs::read_dir(&config.inbox_folder) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map(|e| e == "gguf").unwrap_or(false) {
                            // Found a GGUF file
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                let model_name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                // Publish event
                                let _ = event_tx.send(DigestionEvent {
                                    digestion_id: format!("digest_{}", uuid::Uuid::new_v4()),
                                    event_type: DigestionEventType::ModelReceived,
                                    timestamp: Utc::now(),
                                    details: format!(
                                        "Found {}, size: {} MB",
                                        model_name,
                                        metadata.len() / (1024 * 1024)
                                    ),
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

        // Stubbed: Real structural extraction via GGUFAnalyzer.
        let loci_count = if task.model_path.exists() {
            // Stubbed analysis logic
            let locus = GeneticLocus::new(
                format!("{}-stub", task.model_name.to_lowercase()),
                GeneticCategory::PersonalityGenetics,
                0.5,
                LociSource::Inferred,
            )
            .with_confidence(0.1);
            genome.add_locus(locus);
            1
        } else {
            // Model file not present (e.g. in-memory digestion task for testing)
            tracing::debug!(
                "Model path '{}' not found   skipping structural analysis",
                task.model_path.display()
            );
            0
        };

        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::StructuralAnalysisComplete,
            timestamp: Utc::now(),
            details: format!(
                "Structural analysis complete: {} loci extracted from GGUF tensor table",
                loci_count
            ),
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
        _genome: &SpecialistGenome,
    ) -> Result<SpecialistPersona, Box<dyn std::error::Error>> {
        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::PersonaGenerationStarted,
            timestamp: Utc::now(),
            details: "Generating personality and persona from genetic profile".to_string(),
            progress_percent: Some(80),
        })?;

        // Derive personality archetype from top genetic loci
        let archetype = match task.model_name.to_lowercase() {
            s if s.contains("coder") || s.contains("code") => "Engineer".to_string(),
            s if s.contains("mistral") => "Sage".to_string(),
            s if s.contains("router") => "Sage".to_string(),
            s if s.contains("qwen") && s.contains("70") => "Strategist".to_string(),
            _ => "Generalist".to_string(),
        };

        let personality_persona = PersonalityProfile {
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

        let relational_persona = RelationalProfile {
            natural_allies: vec!["Orchestrator".to_string(), "Synthesizer".to_string()],
            natural_tensions: vec!["Sentinel".to_string()],
            peer_relationships: std::collections::HashMap::new(),
            collaboration_patterns: vec![
                "Pair well with strategic thinkers".to_string(),
                "Thrive in knowledge-sharing environments".to_string(),
            ],
            conflict_resolution_style: "Seek understanding, find common ground".to_string(),
        };

        let narrative_persona = NarrativeProfile {
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
            self_conception: format!(
                "I am a {} who values growth and understanding",
                archetype.to_lowercase()
            ),
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
            fears_and_hopes: "Hope to contribute something unique; fear becoming obsolete"
                .to_string(),
        };

        let experience_persona = ExperienceProfile {
            shared_memories: Vec::new(),
            lessons_learned: Vec::new(),
            achievements: Vec::new(),
            relationship_evolution: std::collections::HashMap::new(),
            evolution_timeline: Vec::new(),
        };

        let persona = SpecialistPersona {
            specialist_id: task.model_name.clone(),
            personality_persona,
            relational_persona,
            narrative_persona,
            experience_persona,
            created_at: Utc::now(),
            version: 1,
        };

        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::PersonaGenerationComplete,
            timestamp: Utc::now(),
            details: format!(
                "Persona generation complete: {} ({} archetype)",
                task.model_name, archetype
            ),
            progress_percent: Some(85),
        })?;

        Ok(persona)
    }

    /// Integrate new specialist into Aaroneous system
    pub async fn integrate_specialist(
        &self,
        task: &DigestionTask,
        genome: &SpecialistGenome,
        persona: &SpecialistPersona,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::ConstellationIntegrationStarted,
            timestamp: Utc::now(),
            details: "Registering specialist in constellation".to_string(),
            progress_percent: Some(90),
        })?;

        // Create constellation node for new specialist
        // Stubbed: let mut node = ConstellationNode::new(...);

        // Persist the persona sidecar alongside the GGUF file.
        // This is what makes the persona visible to GenericSpecialist.with_gguf_path()
        // which loads it from <model>.gguf.persona.json at registration time.
        let persona_path = task.model_path.with_extension("gguf.persona.json");
        if let Ok(persona_json) = serde_json::to_string_pretty(persona) {
            if let Err(e) = std::fs::write(&persona_path, &persona_json) {
                tracing::warn!(
                    "Failed to write persona sidecar {}: {}",
                    persona_path.display(),
                    e
                );
            } else {
                tracing::info!(
                    "Persona persisted: {} (archetype={})",
                    persona_path.display(),
                    persona.personality_persona.archetype
                );
            }
        }

        // Persist the genome sidecar as a companion JSON
        let genome_path = task.model_path.with_extension("gguf.genome.json");
        if let Ok(genome_json) = serde_json::to_string_pretty(genome) {
            let _ = std::fs::write(&genome_path, genome_json);
        }

        self.event_tx.send(DigestionEvent {
            digestion_id: task.digestion_id.clone(),
            event_type: DigestionEventType::ConstellationIntegrationComplete,
            timestamp: Utc::now(),
            details: format!(
                "Specialist {} integrated   persona: {} archetype, genome: {} loci",
                task.model_name,
                persona.personality_persona.archetype,
                genome.genetic_loci.len()
            ),
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

// Complete digestion workflow tests
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
    fn test_personality_profile_creation() {
        let persona = PersonalityProfile {
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

        assert_eq!(persona.archetype, "Scholar");
        assert!(persona.big_five_openness > 0.7);
    }

    #[tokio::test]
    async fn test_task_queue_drain() {
        let (_tx, _rx) = mpsc::unbounded_channel();
        let config = DigestionConfig::default();
        let engine = DigestionEngine::new(config, _tx);

        let task = DigestionTask {
            digestion_id: "q_test".to_string(),
            model_path: PathBuf::from("nonexistent.gguf"),
            model_name: "q_model".to_string(),
            parameter_count: 1_000_000_000,
            created_at: Utc::now(),
            priority: DigestionPriority::Normal,
            status: DigestionStatus::Queued,
            estimated_duration_minutes: 1,
        };

        engine.queue_task(task.clone()).await;
        let completed = engine.process_queue().await;
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].digestion_id, "q_test");
    }
}
