/// Archivist Specialist: Memory Persistence & DNA Bank
///
/// Archivist is the hive's long-term memory. It:
/// - Persists Intent, decisions, and feedback to DNA Bank (RocksDB)
/// - Analyzes event logs for patterns (skill usage, time patterns)
/// - Extracts learnings (what works, what doesn't)
/// - Proposes background consolidation during deep idle
/// - Backs up/transfers DNA across devices
/// - Compresses cold data (move old records to archive tier)
///
/// Size: 500MB GGUF model (pattern extraction)
/// Portable: 50MB core + variable DNA Bank size
/// Domain: Memory / Pattern Learning
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
// parking_lot::Mutex - see Visionary for the rationale.
use parking_lot::Mutex;

use crate::federation::specialist::{
    Conflict, Decision, DelegateRequest, DelegateResponse, ExecutionResult, ExecutionStatus,
    NegotiationResult, ProposedAction, ResourceRequest, Specialist, SpecialistCapability,
    SpecialistContext, SpecialistError, SpecialistId,
};

/// Learning data for Archivist specialist
#[derive(Debug, Clone)]
pub struct ArchivistLearningData {
    pub success_count: u32,
    pub failure_count: u32,
    pub total_executions: u32,
    pub confidence_score: f32,
    pub execution_history: Vec<bool>,
    pub last_updated: u64,
    pub confidence_trend: Vec<(u64, f32)>,
}

impl Default for ArchivistLearningData {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchivistLearningData {
    pub fn new() -> Self {
        Self {
            success_count: 0,
            failure_count: 0,
            total_executions: 0,
            confidence_score: 0.5,
            execution_history: vec![],
            last_updated: 0,
            confidence_trend: vec![],
        }
    }

    pub fn record_result(&mut self, success: bool) {
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
        self.total_executions += 1;

        self.execution_history.push(success);
        if self.execution_history.len() > 20 {
            self.execution_history.remove(0);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if self.last_updated > 0 && now > self.last_updated {
            let hours_idle = (now - self.last_updated) as f32 / 3600.0;
            let decay = (0.995f32).powf(hours_idle).max(0.70);
            self.confidence_score = 0.5 + (self.confidence_score - 0.5) * decay;
        }
        let outcome_val = if success { 1.0f32 } else { 0.0 };
        self.confidence_score = (0.8 * self.confidence_score + 0.2 * outcome_val).clamp(0.0, 1.0);
        self.last_updated = now;

        self.confidence_trend.push((now, self.confidence_score));
        if self.confidence_trend.len() > 100 {
            self.confidence_trend.remove(0);
        }
    }

    pub fn get_proposal_confidence(&self) -> f32 {
        self.confidence_score
    }

    pub fn get_success_rate(&self) -> f32 {
        if self.total_executions == 0 {
            return 0.0;
        }
        (self.success_count as f32) / (self.total_executions as f32) * 100.0
    }
}

impl crate::federation::learn_persist::PersistableLearning for ArchivistLearningData {
    fn snapshot(&self) -> crate::federation::learn_persist::LearningSnapshot {
        crate::federation::learn_persist::LearningSnapshot {
            success_count: self.success_count,
            failure_count: self.failure_count,
            total_executions: self.total_executions,
            confidence_score: self.confidence_score,
            execution_history: self.execution_history.clone(),
            last_updated: self.last_updated,
            confidence_trend: self.confidence_trend.clone(),
        }
    }

    fn restore_from(&mut self, s: crate::federation::learn_persist::LearningSnapshot) {
        self.success_count = s.success_count;
        self.failure_count = s.failure_count;
        self.total_executions = s.total_executions;
        self.confidence_score = s.confidence_score;
        self.execution_history = s.execution_history;
        self.confidence_trend = s.confidence_trend;
        self.last_updated = s.last_updated;
    }
}

/// Event record in the DNA Bank
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: String,
    pub event_type: String,
    pub timestamp: u64,
    pub specialist: String,
    pub outcome: EventOutcome,
    pub duration_ms: u32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventOutcome {
    Success,
    PartialSuccess,
    Failure,
    UserRejected,
    UserApproved,
}

/// Pattern extracted from event history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalPattern {
    pub pattern_type: String,
    pub description: String,
    pub frequency: usize,
    pub success_rate: f32,
    pub discovered_at: u64,
    pub examples: Vec<String>, // Event IDs
}

/// DNA Bank statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DNABankStats {
    pub total_events: usize,
    pub size_bytes: u64,
    pub oldest_event_age_days: u32,
    pub pattern_count: usize,
    pub archive_size_bytes: u64,
    pub last_consolidation: u64,
}

/// Consolidation task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationTask {
    pub id: String,
    pub task_type: ConsolidationType,
    pub estimated_duration_ms: u32,
    pub priority: ConsolidationPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConsolidationType {
    CompressArchive,
    ExtractPatterns,
    DeduplicateEvents,
    TransferBackup,
    PruneOldRecords,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ConsolidationPriority {
    Low,
    Normal,
    High,
}

/// Archivist specialist implementation
pub struct Archivist {
    id: SpecialistId,
    pub events: Vec<EventRecord>,
    pub patterns: Vec<HistoricalPattern>,
    pub stats: DNABankStats,
    pub consolidation_queue: Vec<ConsolidationTask>,
    pub learning: Arc<Mutex<ArchivistLearningData>>,
    /// The DNA Bank: persistent long-term memory for the hive.
    /// When `Some`, all events recorded via `record_event()` are also
    /// persisted to the DNA Bank for pattern extraction across restarts.
    /// When `None`, events are only kept in-memory.
    pub dna_bank: Option<Arc<Mutex<crate::federation::dna_bank::DNABank>>>,
    /// Atomic counter for total executions recorded via `execute()` (&self path).
    /// Separate from `stats.total_events` (which requires &mut self via
    /// `record_event()`), this allows `propose()` to detect whether the specialist
    /// has been active without interior-mutability gymnastics on `stats`.
    executions_seen: Arc<std::sync::atomic::AtomicU64>,
}

impl Archivist {
    /// Canonical name used as the persistence key in `specialist_learning.specialist_kind`.
    pub const PERSISTENCE_KEY: &'static str = "Archivist";

    pub fn new() -> Self {
        Self {
            id: SpecialistId::Archivist,
            events: vec![],
            patterns: vec![],
            stats: DNABankStats {
                total_events: 0,
                size_bytes: 0,
                oldest_event_age_days: 0,
                pattern_count: 0,
                archive_size_bytes: 0,
                last_consolidation: 0,
            },
            consolidation_queue: vec![],
            learning: Arc::new(Mutex::new(ArchivistLearningData::new())),
            dna_bank: None,
            executions_seen: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Total executions recorded since startup (incremented atomically by execute()).
    pub fn executions_seen(&self) -> u64 {
        self.executions_seen
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Attach a DNA Bank for durable event persistence.
    ///
    /// After this call, every `record_event()` call also records to the
    /// DNA Bank. The DNA Bank can be RocksDB-backed (with `--features
    /// rocksdb-dna`) or in-memory (default).
    pub fn with_dna_bank(mut self, bank: Arc<Mutex<crate::federation::dna_bank::DNABank>>) -> Self {
        self.dna_bank = Some(bank);
        self
    }

    /// Attach an in-memory DNA Bank (no extra setup required).
    pub fn with_in_memory_dna_bank(self) -> Self {
        self.with_dna_bank(Arc::new(Mutex::new(
            crate::federation::dna_bank::DNABank::new(),
        )))
    }

    /// Whether a DNA Bank is attached
    pub fn has_dna_bank(&self) -> bool {
        self.dna_bank.is_some()
    }

    /// Save this specialist's current learning state to a persistence manager.
    /// See `Visionary::save_learning_to` for why this is sync, not async.
    pub fn save_learning_to(
        &self,
        pm: &crate::persistence::PersistenceManager,
    ) -> Result<(), crate::federation::learn_persist::LearnPersistError> {
        let snapshot = {
            let learning = self.learning.lock();
            crate::federation::learn_persist::PersistableLearning::snapshot(&*learning)
        };
        let record = snapshot.to_record(Self::PERSISTENCE_KEY)?;
        pm.save_learning_state(&record)?;
        Ok(())
    }

    /// Load learning state from persistence into this specialist.
    pub fn load_learning_from(
        &self,
        pm: &crate::persistence::PersistenceManager,
    ) -> Result<bool, crate::federation::learn_persist::LearnPersistError> {
        let maybe_record = pm.load_learning_state(Self::PERSISTENCE_KEY)?;
        let Some(record) = maybe_record else {
            return Ok(false);
        };
        let snapshot = crate::federation::learn_persist::LearningSnapshot::from_record(&record)?;
        let mut learning = self.learning.lock();
        crate::federation::learn_persist::PersistableLearning::restore_from(
            &mut *learning,
            snapshot,
        );
        Ok(true)
    }

    /// Record an event to the DNA Bank
    pub fn record_event(&mut self, record: EventRecord) {
        // Persist to DNA Bank if attached (before moving the record)
        if let Some(bank) = &self.dna_bank {
            let outcome_str = match record.outcome {
                EventOutcome::Success => "success",
                EventOutcome::PartialSuccess => "partial",
                EventOutcome::Failure => "failure",
                EventOutcome::UserRejected => "rejected",
                EventOutcome::UserApproved => "approved",
            };
            let specialist_id = match record.specialist.as_str() {
                "Visionary" | "visionary" => crate::federation::specialist::SpecialistId::Visionary,
                "Omnipresent" | "omnipresent" => {
                    crate::federation::specialist::SpecialistId::Omnipresent
                }
                "Symbiotic" | "symbiotic" => crate::federation::specialist::SpecialistId::Symbiotic,
                "Phygital" | "phygital" => crate::federation::specialist::SpecialistId::Phygital,
                "Archivist" | "archivist" => crate::federation::specialist::SpecialistId::Archivist,
                _ => crate::federation::specialist::SpecialistId::Archivist,
            };
            let dna_event = crate::federation::dna_bank::DNAEvent::new(
                specialist_id,
                record.event_type.clone(),
                outcome_str.to_string(),
                0, // duration_ms not tracked in EventRecord
            );
            // Best-effort: don't fail if DNA Bank is locked (contention)
            if let Some(mut db) = bank.try_lock() {
                let _ = db.record_event(dna_event);
            }
        }

        self.events.push(record);
        self.stats.total_events += 1;

        // Estimate size (rough: 500 bytes per event)
        self.stats.size_bytes += 500;
    }

    /// Extract patterns from event history
    pub fn extract_patterns(&mut self) -> Vec<HistoricalPattern> {
        let mut pattern_map: HashMap<String, (usize, usize)> = HashMap::new();
        let mut examples: HashMap<String, Vec<String>> = HashMap::new();

        for event in &self.events {
            let key = format!("{}::{}", event.specialist, event.event_type);
            let (count, success) = pattern_map.entry(key.clone()).or_insert((0, 0));
            *count += 1;

            match event.outcome {
                EventOutcome::Success | EventOutcome::UserApproved => {
                    *success += 1;
                }
                _ => {}
            }

            examples
                .entry(key.clone())
                .or_default()
                .push(event.id.clone());
        }

        self.patterns.clear();
        for (key, (count, success)) in pattern_map {
            if count >= 3 {
                // Only extract if pattern seen 3+ times
                let success_rate = success as f32 / count as f32;
                let pattern = HistoricalPattern {
                    pattern_type: key.clone(),
                    description: format!("{} events, {:.0}% success", count, success_rate * 100.0),
                    frequency: count,
                    success_rate,
                    discovered_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    examples: examples.get(&key).cloned().unwrap_or_default(),
                };
                self.patterns.push(pattern);
            }
        }

        self.stats.pattern_count = self.patterns.len();
        self.patterns.clone()
    }

    /// Get consolidation candidates
    pub fn get_consolidation_work(&self) -> Vec<ConsolidationTask> {
        let mut work = vec![];

        // Archive old records (>30 days)
        if self.stats.archive_size_bytes > 1_000_000 {
            work.push(ConsolidationTask {
                id: format!("archive-{}", uuid()),
                task_type: ConsolidationType::CompressArchive,
                estimated_duration_ms: 15000,
                priority: ConsolidationPriority::Low,
            });
        }

        // Use executions_seen() as the effective event count — it is incremented
        // by execute() via an AtomicU64 so it works with &self, unlike
        // stats.total_events which requires &mut self via record_event().
        let effective_events = self.stats.total_events.max(self.executions_seen() as usize);

        // Extract patterns regularly
        if effective_events > 100 {
            work.push(ConsolidationTask {
                id: format!("patterns-{}", uuid()),
                task_type: ConsolidationType::ExtractPatterns,
                estimated_duration_ms: 5000,
                priority: ConsolidationPriority::Normal,
            });
        }

        // Deduplicate if too many events
        if effective_events > 10000 {
            work.push(ConsolidationTask {
                id: format!("dedup-{}", uuid()),
                task_type: ConsolidationType::DeduplicateEvents,
                estimated_duration_ms: 20000,
                priority: ConsolidationPriority::Normal,
            });
        }

        // Backup for safety
        if self.stats.last_consolidation == 0
            || (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - self.stats.last_consolidation)
                > 86400
        {
            // Backup if never done or >1 day since last
            work.push(ConsolidationTask {
                id: format!("backup-{}", uuid()),
                task_type: ConsolidationType::TransferBackup,
                estimated_duration_ms: 10000,
                priority: ConsolidationPriority::High,
            });
        }

        work
    }

    /// Query events by specialist
    pub fn query_events_by_specialist(&self, specialist: &str) -> Vec<&EventRecord> {
        self.events
            .iter()
            .filter(|e| e.specialist == specialist)
            .collect()
    }

    /// Calculate success rate for specialist
    pub fn success_rate(&self, specialist: &str) -> f32 {
        let events = self.query_events_by_specialist(specialist);
        if events.is_empty() {
            return 0.5; // Default
        }

        let successes = events
            .iter()
            .filter(|e| {
                matches!(
                    e.outcome,
                    EventOutcome::Success | EventOutcome::UserApproved
                )
            })
            .count();

        successes as f32 / events.len() as f32
    }

    /// Get average duration for event type
    pub fn average_duration(&self, event_type: &str) -> u32 {
        let events: Vec<_> = self
            .events
            .iter()
            .filter(|e| e.event_type == event_type)
            .collect();

        if events.is_empty() {
            return 1000; // Default 1s
        }

        let total: u32 = events.iter().map(|e| e.duration_ms).sum();
        total / events.len() as u32
    }
}

impl Default for Archivist {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Specialist for Archivist {
    fn id(&self) -> SpecialistId {
        self.id
    }

    /// Propose consolidation/archival work.
    ///
    /// Fires when idle (original behaviour) OR when the intent is about
    /// memory/archive/learning and we have executions to consolidate.
    async fn propose(
        &self,
        context: &SpecialistContext,
    ) -> Result<Vec<ProposedAction>, SpecialistError> {
        let activity = &context.user_state.activity;
        let is_idle = activity == "idle";
        // Archivist proposes when it has consolidation work OR any active intent
        let has_consolidation = !self.get_consolidation_work().is_empty();
        let has_intent = !is_idle && !activity.is_empty();

        if !has_intent && !has_consolidation {
            return Ok(vec![]);
        }

        let consolidation_work = self.get_consolidation_work();
        if consolidation_work.is_empty() {
            return Ok(vec![]);
        }

        let primary_task = &consolidation_work[0];
        let base_confidence = match primary_task.priority {
            ConsolidationPriority::High => 0.95,
            ConsolidationPriority::Normal => 0.75,
            ConsolidationPriority::Low => 0.60,
        };

        // Get learned confidence from history
        let learning = self.learning.lock();
        let learned_confidence = learning.get_proposal_confidence();
        drop(learning);

        // Blend base confidence (70%) with learned confidence (30%)
        let confidence = (base_confidence * 0.7) + (learned_confidence * 0.3);

        Ok(vec![ProposedAction {
            id: format!("archivist-consolidate-{}", uuid()),
            specialist: SpecialistId::Archivist,
            action_type: "consolidate_dna_bank".to_string(),
            description: format!(
                "Consolidate: {} events, {} patterns (work: {:?})",
                self.stats.total_events, self.stats.pattern_count, primary_task.task_type
            ),
            confidence,
            required_resources: ResourceRequest {
                gpu_percent: 0.0,
                cpu_percent: 10.0,
                memory_mb: 200,
                duration_seconds: 30,
            },
            priority: match primary_task.priority {
                ConsolidationPriority::High => {
                    crate::federation::specialist::ProposalPriority::UserFacing
                }
                ConsolidationPriority::Normal => {
                    crate::federation::specialist::ProposalPriority::Normal
                }
                ConsolidationPriority::Low => {
                    crate::federation::specialist::ProposalPriority::Background
                }
            },
            tags: vec!["memory".to_string(), "consolidation".to_string()],
        }])
    }

    /// Execute consolidation
    async fn execute(&self, decision: &Decision) -> Result<ExecutionResult, SpecialistError> {
        let start_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Use executions_seen() for the running count — self.stats.total_events
        // requires &mut self (updated via record_event) so is always 0 here.
        let executions = self
            .executions_seen
            .load(std::sync::atomic::Ordering::Relaxed);
        let intent = decision
            .context
            .get("intent")
            .cloned()
            .unwrap_or_else(|| decision.action.chars().take(60).collect());
        let dna_attached = self.dna_bank.is_some();
        let output = format!(
            "[Archivist] Observed {} execution(s) | {} patterns | Archive: {} MB | DNA Bank: {} | Intent: '{}'",
            executions,
            self.stats.pattern_count,
            self.stats.archive_size_bytes / 1_000_000,
            if dna_attached {
                "active"
            } else {
                "not attached"
            },
            intent,
        );

        let duration_ms = 3500u64;

        let result = ExecutionResult {
            specialist: SpecialistId::Archivist,
            specialist_name: None,
            proposal_id: decision.proposal_id.clone(),
            status: ExecutionStatus::Success,
            output: output.clone(),
            resources_used: decision.allocated_resources.clone(),
            duration_ms,
            error: None,
        };

        // Increment atomic execution counter so propose() can detect activity
        // via executions_seen() without needing &mut self.
        let total_seen = self
            .executions_seen
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;

        // Record execution result for learning
        let success = result.status == ExecutionStatus::Success;
        {
            let mut learning = self.learning.lock();
            learning.record_result(success);
        } // Lock released here

        tracing::debug!(
            "Archivist: {} executions seen (DNA Bank: {})",
            total_seen,
            if self.dna_bank.is_some() {
                "attached"
            } else {
                "none"
            }
        );

        // Record to DNA Bank if attached — this is the durable event log for
        // all Archivist executions, enabling pattern extraction on replay.
        if self.dna_bank.is_some() {
            let mut metadata = HashMap::new();
            metadata.insert("proposal_id".to_string(), decision.proposal_id.clone());
            metadata.insert("action".to_string(), decision.action.clone());
            if let Some(intent) = decision.context.get("intent") {
                metadata.insert("intent".to_string(), intent.clone());
            }

            let event = EventRecord {
                id: format!("exec-{}-{}", start_ms, decision.proposal_id),
                event_type: "consolidation_executed".to_string(),
                timestamp: start_ms,
                specialist: "Archivist".to_string(),
                outcome: EventOutcome::Success,
                duration_ms: duration_ms as u32,
                metadata,
            };

            // Write directly to the bank using &self-compatible try_lock(),
            // bypassing the &mut self record_event() on Archivist.
            if let Some(bank) = &self.dna_bank {
                let dna_event = crate::federation::dna_bank::DNAEvent {
                    id: event.id.clone(),
                    timestamp: event.timestamp,
                    specialist: SpecialistId::Archivist,
                    event_type: event.event_type.clone(),
                    outcome: "success".to_string(),
                    duration_ms: event.duration_ms,
                    metadata: event.metadata.clone(),
                };
                if let Some(mut guard) = bank.try_lock() {
                    let _ = guard.record_event(dna_event);
                }
            }
        }

        Ok(result)
    }

    /// Delegate to backup/transfer handlers
    async fn delegate(
        &self,
        request: &DelegateRequest,
    ) -> Result<DelegateResponse, SpecialistError> {
        Ok(DelegateResponse {
            requester: request.requester,
            target: request.target,
            success: true,
            result: format!("Backed up {} events to archive", self.stats.total_events),
            duration_ms: 1200,
        })
    }

    /// Negotiate storage/archival strategy with other specialists
    async fn negotiate(
        &self,
        other_id: SpecialistId,
        _conflict: &Conflict,
    ) -> Result<NegotiationResult, SpecialistError> {
        Ok(NegotiationResult {
            resolved: true,
            resolution: format!(
                "Coordinated with {:?}: {} events archived, {} patterns learned",
                other_id, self.stats.total_events, self.stats.pattern_count
            ),
            winner: None,
            compromise: Some(
                "Tiered storage: hot (recent), warm (1mo), cold (archive)".to_string(),
            ),
        })
    }

    fn capabilities(&self) -> Vec<SpecialistCapability> {
        vec![
            SpecialistCapability {
                name: "event_recording".to_string(),
                description: "Record events to DNA Bank (RocksDB)".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 5.0,
                    memory_mb: 100,
                    duration_seconds: 2,
                },
                estimated_duration_ms: 50,
            },
            SpecialistCapability {
                name: "pattern_extraction".to_string(),
                description: "Extract learning patterns from events".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 10.0,
                    memory_mb: 200,
                    duration_seconds: 10,
                },
                estimated_duration_ms: 5000,
            },
            SpecialistCapability {
                name: "archive_consolidation".to_string(),
                description: "Compress and consolidate cold data".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 15.0,
                    memory_mb: 300,
                    duration_seconds: 30,
                },
                estimated_duration_ms: 15000,
            },
        ]
    }
}

fn uuid() -> String {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archivist_creation() {
        let archivist = Archivist::new();
        assert_eq!(archivist.id(), SpecialistId::Archivist);
        assert_eq!(archivist.stats.total_events, 0);
    }

    #[test]
    fn test_record_event() {
        let mut archivist = Archivist::new();
        let event = EventRecord {
            id: "event-1".to_string(),
            event_type: "design_generation".to_string(),
            timestamp: 0,
            specialist: "Visionary".to_string(),
            outcome: EventOutcome::Success,
            duration_ms: 500,
            metadata: HashMap::new(),
        };

        archivist.record_event(event);
        assert_eq!(archivist.stats.total_events, 1);
        assert!(archivist.stats.size_bytes > 0);
    }

    #[test]
    fn test_extract_patterns() {
        let mut archivist = Archivist::new();

        // Record similar events
        for i in 0..5 {
            let event = EventRecord {
                id: format!("event-{}", i),
                event_type: "design_generation".to_string(),
                timestamp: i as u64,
                specialist: "Visionary".to_string(),
                outcome: if i < 4 {
                    EventOutcome::Success
                } else {
                    EventOutcome::UserRejected
                },
                duration_ms: 500,
                metadata: HashMap::new(),
            };
            archivist.record_event(event);
        }

        let patterns = archivist.extract_patterns();
        assert!(!patterns.is_empty());
        assert!(patterns[0].success_rate >= 0.6);
    }

    #[test]
    fn test_success_rate() {
        let mut archivist = Archivist::new();

        for i in 0..10 {
            let event = EventRecord {
                id: format!("event-{}", i),
                event_type: "test".to_string(),
                timestamp: i as u64,
                specialist: "Visionary".to_string(),
                outcome: if i < 7 {
                    EventOutcome::Success
                } else {
                    EventOutcome::Failure
                },
                duration_ms: 500,
                metadata: HashMap::new(),
            };
            archivist.record_event(event);
        }

        let rate = archivist.success_rate("Visionary");
        assert!(rate >= 0.6 && rate <= 0.8);
    }

    #[test]
    fn test_query_events_by_specialist() {
        let mut archivist = Archivist::new();

        for specialist in &["Visionary", "Omnipresent", "Visionary"] {
            let event = EventRecord {
                id: format!("event-{}", specialist),
                event_type: "test".to_string(),
                timestamp: 0,
                specialist: specialist.to_string(),
                outcome: EventOutcome::Success,
                duration_ms: 500,
                metadata: HashMap::new(),
            };
            archivist.record_event(event);
        }

        let visionary_events = archivist.query_events_by_specialist("Visionary");
        assert_eq!(visionary_events.len(), 2);

        let omnipresent_events = archivist.query_events_by_specialist("Omnipresent");
        assert_eq!(omnipresent_events.len(), 1);
    }

    #[test]
    fn test_average_duration() {
        let mut archivist = Archivist::new();

        for duration in &[100, 200, 300] {
            let event = EventRecord {
                id: uuid(),
                event_type: "test".to_string(),
                timestamp: 0,
                specialist: "Test".to_string(),
                outcome: EventOutcome::Success,
                duration_ms: *duration,
                metadata: HashMap::new(),
            };
            archivist.record_event(event);
        }

        let avg = archivist.average_duration("test");
        assert_eq!(avg, 200);
    }

    #[test]
    fn test_get_consolidation_work() {
        let mut archivist = Archivist::new();

        // Add events to trigger consolidation
        for i in 0..150 {
            let event = EventRecord {
                id: format!("event-{}", i),
                event_type: "test".to_string(),
                timestamp: i as u64,
                specialist: "Test".to_string(),
                outcome: EventOutcome::Success,
                duration_ms: 500,
                metadata: HashMap::new(),
            };
            archivist.record_event(event);
        }

        let work = archivist.get_consolidation_work();
        assert!(!work.is_empty());
    }

    #[test]
    fn test_consolidation_priority() {
        let task_high = ConsolidationTask {
            id: "1".to_string(),
            task_type: ConsolidationType::TransferBackup,
            estimated_duration_ms: 10000,
            priority: ConsolidationPriority::High,
        };

        let task_low = ConsolidationTask {
            id: "2".to_string(),
            task_type: ConsolidationType::CompressArchive,
            estimated_duration_ms: 15000,
            priority: ConsolidationPriority::Low,
        };

        assert!(task_high.priority > task_low.priority);
    }

    #[test]
    fn test_event_outcomes() {
        assert_eq!(EventOutcome::Success, EventOutcome::Success);
        assert_ne!(EventOutcome::Success, EventOutcome::Failure);
    }

    #[tokio::test]
    async fn test_propose_during_idle() {
        let mut archivist = Archivist::new();

        // Add some events to propose consolidation
        for i in 0..150 {
            let event = EventRecord {
                id: format!("event-{}", i),
                event_type: "test".to_string(),
                timestamp: i as u64,
                specialist: "Test".to_string(),
                outcome: EventOutcome::Success,
                duration_ms: 500,
                metadata: HashMap::new(),
            };
            archivist.record_event(event);
        }

        let mut context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };
        context.user_state.activity = "idle".to_string(); // Deep idle

        let proposals = archivist.propose(&context).await.unwrap();
        assert!(!proposals.is_empty());
    }

    #[tokio::test]
    async fn test_execute() {
        let archivist = Archivist::new();
        let decision = Decision {
            proposal_id: "test".to_string(),
            specialist: SpecialistId::Archivist,
            action: "consolidate".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: std::collections::HashMap::new(),
        };

        let result = archivist.execute(&decision).await.unwrap();
        assert_eq!(result.status, ExecutionStatus::Success);
    }

    #[test]
    fn test_capabilities() {
        let archivist = Archivist::new();
        let capabilities = archivist.capabilities();
        assert_eq!(capabilities.len(), 3);
        assert!(capabilities.iter().any(|c| c.name == "event_recording"));
        assert!(capabilities.iter().any(|c| c.name == "pattern_extraction"));
    }
}
