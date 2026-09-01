use crate::federation::specialist::{ProposalPriority, SpecialistId};
/// The Intent type — the foundational object of the Aaroneous pipeline.
///
/// An `Intent` represents a user's goal or task as it flows through the
/// federation. It is:
/// - Created when a user states what they want (via CLI, API, or voice)
/// - Versioned so Omnipresent can track which device has the latest version
/// - Scaled by Symbiotic based on biometric state (stress, fatigue)
/// - Interpreted by Visionary to generate design variants
/// - Anchored by Phygital in physical space via AR landmarks
/// - Archived by Archivist in the ArtifactRegistry for pattern learning
/// - Arbitrated by Sentinel when multiple intents compete for resources
///
/// # Examples
///
/// ```
/// use a_run::federation::intent::Intent;
/// use a_run::federation::IntentPriority;
///
/// let intent = Intent::new("dashboard redesign")
///     .with_priority(IntentPriority::High)
///     .with_tag("ui")
///     .with_context("target_device", "desktop");
/// ```
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core intent object — the user's goal as it flows through the federation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Intent {
    /// Stable identifier — survives versioning
    pub id: String,
    /// Version counter — incremented each time the intent is updated.
    /// Omnipresent uses this to detect drift between devices.
    pub version: u32,
    /// The natural-language goal. This is what the user said/typed.
    /// Examples: "redesign the dashboard", "sync to phone before meeting"
    pub content: String,
    /// How urgently this intent needs to be addressed
    pub priority: IntentPriority,
    /// Current lifecycle state
    pub status: IntentStatus,
    /// Which specialist is currently the primary executor (None = not yet assigned)
    pub assigned_to: Option<SpecialistId>,
    /// Arbitrary context key/values (device hints, user preferences, metadata)
    pub context: HashMap<String, String>,
    /// Searchable tags
    pub tags: Vec<String>,
    /// Unix timestamp (seconds) when this intent was created
    pub created_at: u64,
    /// Unix timestamp (seconds) when this intent was last updated
    pub updated_at: u64,
    /// Source of the intent: CLI input, API request, voice, etc.
    pub source: IntentSource,
    /// Scaling applied by Symbiotic (None = not yet scaled)
    pub scaling: Option<IntentScaling>,
    /// Results produced by executing specialists
    pub results: Vec<IntentResult>,
}

/// How urgently the federation should address this intent
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntentPriority {
    Background = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl From<IntentPriority> for ProposalPriority {
    fn from(p: IntentPriority) -> Self {
        match p {
            IntentPriority::Background => ProposalPriority::Background,
            IntentPriority::Normal => ProposalPriority::Normal,
            IntentPriority::High => ProposalPriority::UserFacing,
            IntentPriority::Critical => ProposalPriority::UserFacing,
        }
    }
}

/// Lifecycle state of an intent
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentStatus {
    /// Just created, not yet seen by the federation
    Pending,
    /// Picked up by Sentinel for arbitration
    Arbitrating,
    /// Assigned to a specialist for execution
    Executing,
    /// At least one result produced; may still have more specialists working
    PartialResult,
    /// All assigned specialists have completed
    Completed,
    /// Deferred by Symbiotic due to user stress/fatigue
    Deferred,
    /// Superseded by a newer version
    Superseded,
    /// Explicitly cancelled
    Cancelled,
    /// Failed — no specialist could execute successfully
    Failed,
}

/// Where this intent originated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentSource {
    /// Typed into the CLI (`aaroneous intent "..."`)
    Cli,
    /// HTTP POST to `/intent`
    Api,
    /// Received via P2P sync from another device
    Sync { from_device: String },
    /// Inferred from biometric state (e.g., Symbiotic detected focus → generated intent)
    Inferred,
    /// Loaded from the ArtifactRegistry as a recurring pattern
    DnaPattern { pattern_id: String },
}

/// How Symbiotic wants to scale this intent's execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentScaling {
    /// Seconds to delay before starting execution (0 = immediate)
    pub delay_seconds: u32,
    /// Maximum duration for execution (minutes)
    pub max_duration_minutes: u32,
    /// Whether to allow interruptions during execution
    pub allow_interruption: bool,
    /// Reason for the scaling (displayed to user)
    pub reason: String,
}

/// A result produced by a specialist executing this intent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentResult {
    /// Which specialist produced this
    pub from: SpecialistId,
    /// Human-readable summary
    pub summary: String,
    /// Structured output (JSON, design spec, etc.) — optional
    pub payload: Option<serde_json::Value>,
    /// When this result was produced (Unix seconds)
    pub produced_at: u64,
    /// Whether this result was accepted by the user
    pub accepted: Option<bool>,
}

impl Intent {
    /// Create a new intent with the given natural-language content.
    /// Generates a UUID and sets status to `Pending`.
    pub fn new(content: impl Into<String>) -> Self {
        let now = now_secs();
        Self {
            id: format!("intent-{}", uuid_hex()),
            version: 1,
            content: content.into(),
            priority: IntentPriority::Normal,
            status: IntentStatus::Pending,
            assigned_to: None,
            context: HashMap::new(),
            tags: vec![],
            created_at: now,
            updated_at: now,
            source: IntentSource::Cli,
            scaling: None,
            results: vec![],
        }
    }

    /// Set priority (builder)
    pub fn with_priority(mut self, priority: IntentPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Add a tag (builder)
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add a context key/value (builder)
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Set source (builder)
    pub fn with_source(mut self, source: IntentSource) -> Self {
        self.source = source;
        self
    }

    /// Increment the version and update `updated_at`. Called by Omnipresent
    /// when syncing an updated intent to other devices.
    pub fn bump_version(&mut self) {
        self.version += 1;
        self.updated_at = now_secs();
    }

    /// Apply Symbiotic's scaling recommendation to this intent.
    pub fn apply_scaling(&mut self, scaling: IntentScaling) {
        self.scaling = Some(scaling);
        self.updated_at = now_secs();
    }

    /// Transition to a new status, updating `updated_at`.
    pub fn transition(&mut self, status: IntentStatus) {
        self.status = status;
        self.updated_at = now_secs();
    }

    /// Record a result from a specialist.
    pub fn add_result(&mut self, result: IntentResult) {
        self.results.push(result);
        self.updated_at = now_secs();
        // Auto-advance status if still executing
        if self.status == IntentStatus::Executing {
            self.status = IntentStatus::PartialResult;
        }
    }

    /// Mark the intent as fully completed.
    pub fn complete(&mut self) {
        self.status = IntentStatus::Completed;
        self.updated_at = now_secs();
    }

    /// Serialize this intent to bytes for P2P sync (Omnipresent).
    pub fn to_sync_payload(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Deserialize an intent from a P2P sync payload.
    pub fn from_sync_payload(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Get the intent's content as a string for use in LLM prompts.
    pub fn as_prompt_context(&self) -> String {
        format!(
            "Intent: {}\nPriority: {:?}\nTags: {}\nContext: {}",
            self.content,
            self.priority,
            self.tags.join(", "),
            self.context
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn uuid_hex() -> String {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:032x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_creation() {
        let intent = Intent::new("redesign the dashboard");
        assert_eq!(intent.content, "redesign the dashboard");
        assert_eq!(intent.status, IntentStatus::Pending);
        assert_eq!(intent.version, 1);
        assert_eq!(intent.priority, IntentPriority::Normal);
        assert!(intent.results.is_empty());
        assert!(intent.scaling.is_none());
    }

    #[test]
    fn test_intent_builder() {
        let intent = Intent::new("sync to phone")
            .with_priority(IntentPriority::High)
            .with_tag("sync")
            .with_tag("mobile")
            .with_context("target_device", "iPhone15")
            .with_source(IntentSource::Api);

        assert_eq!(intent.priority, IntentPriority::High);
        assert_eq!(intent.tags, vec!["sync", "mobile"]);
        assert_eq!(
            intent.context.get("target_device"),
            Some(&"iPhone15".to_string())
        );
        assert_eq!(intent.source, IntentSource::Api);
    }

    #[test]
    fn test_intent_version_bump() {
        let mut intent = Intent::new("test");
        assert_eq!(intent.version, 1);
        intent.bump_version();
        assert_eq!(intent.version, 2);
        intent.bump_version();
        assert_eq!(intent.version, 3);
    }

    #[test]
    fn test_intent_status_transitions() {
        let mut intent = Intent::new("test");
        assert_eq!(intent.status, IntentStatus::Pending);
        intent.transition(IntentStatus::Arbitrating);
        assert_eq!(intent.status, IntentStatus::Arbitrating);
        intent.transition(IntentStatus::Executing);
        assert_eq!(intent.status, IntentStatus::Executing);
        intent.complete();
        assert_eq!(intent.status, IntentStatus::Completed);
    }

    #[test]
    fn test_intent_add_result_advances_status() {
        let mut intent = Intent::new("test");
        intent.transition(IntentStatus::Executing);

        intent.add_result(IntentResult {
            from: SpecialistId::Visionary,
            summary: "Generated 3 design variants".to_string(),
            payload: Some(serde_json::json!({"variants": 3})),
            produced_at: 0,
            accepted: None,
        });

        // Status should advance from Executing to PartialResult
        assert_eq!(intent.status, IntentStatus::PartialResult);
        assert_eq!(intent.results.len(), 1);
    }

    #[test]
    fn test_intent_priority_to_proposal_priority() {
        assert_eq!(
            ProposalPriority::from(IntentPriority::Background),
            ProposalPriority::Background
        );
        assert_eq!(
            ProposalPriority::from(IntentPriority::Normal),
            ProposalPriority::Normal
        );
        assert_eq!(
            ProposalPriority::from(IntentPriority::High),
            ProposalPriority::UserFacing
        );
        assert_eq!(
            ProposalPriority::from(IntentPriority::Critical),
            ProposalPriority::UserFacing
        );
    }

    #[test]
    fn test_intent_sync_round_trip() {
        let original = Intent::new("sync test")
            .with_priority(IntentPriority::High)
            .with_tag("sync");

        let bytes = original.to_sync_payload();
        assert!(!bytes.is_empty());

        let recovered = Intent::from_sync_payload(&bytes).unwrap();
        assert_eq!(recovered.id, original.id);
        assert_eq!(recovered.content, original.content);
        assert_eq!(recovered.version, original.version);
        assert_eq!(recovered.priority, original.priority);
    }

    #[test]
    fn test_intent_as_prompt_context() {
        let intent = Intent::new("dashboard redesign")
            .with_tag("ui")
            .with_context("device", "desktop");

        let ctx = intent.as_prompt_context();
        assert!(ctx.contains("dashboard redesign"));
        assert!(ctx.contains("ui"));
        assert!(ctx.contains("device=desktop"));
    }

    #[test]
    fn test_intent_apply_scaling() {
        let mut intent = Intent::new("test");
        intent.apply_scaling(IntentScaling {
            delay_seconds: 300,
            max_duration_minutes: 30,
            allow_interruption: false,
            reason: "User is stressed".to_string(),
        });

        let s = intent.scaling.unwrap();
        assert_eq!(s.delay_seconds, 300);
        assert!(!s.allow_interruption);
    }

    #[test]
    fn test_unique_ids() {
        let a = Intent::new("a");
        let b = Intent::new("b");
        assert_ne!(a.id, b.id);
    }
}
