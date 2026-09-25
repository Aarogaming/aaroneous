//! crates/mcp_server-to-be: the intent-execution backend boundary.
//!
//! `McpService` (in `service.rs`) is a generic MCP/JSON-RPC protocol
//! implementation; everything it needs from "the running system" goes
//! through this one trait, `IntentBackend`, rather than reaching into
//! `Federation`'s fields directly. Two things this buys us:
//!
//! 1. **Today**: `service.rs`'s dozen or so `self.federation`-touching
//!    methods collapse to calls against a documented, typed interface
//!    instead of raw lock-guarded `Vec`/`HashMap` field access spread
//!    across the file — the exact shape of "what MCP needs from the hive"
//!    is now visible in one place (this trait) instead of implicit in
//!    call-site behavior.
//! 2. **Next**: this trait and its associated types are the actual
//!    contract `mcp_service` needs to become crate-portable. Cargo forbids
//!    circular package dependencies — `crates/mcp_server` depending on
//!    `hypervisor`'s lib while `hypervisor`'s own binary depends on
//!    `mcp_server` is a cycle Cargo rejects outright — so a standalone MCP
//!    crate cannot name `Federation` (or any other hypervisor-internal
//!    type) in its own signatures. Once this trait's types are Federation-
//!    agnostic, moving `mcp_service` into its own crate is a matter of
//!    relocating this trait's *definition* there and leaving only its
//!    `impl IntentBackend for Federation` behind in hypervisor, which is
//!    exactly the shape a downstream crate consuming an upstream one is
//!    supposed to take.
//!
//! See `docs/TECH_DEBT_TEST_DUPLICATION.md` §2.2 for the fuller writeup of
//! why `crates/mcp_server` (today a stale, unreferenced duplicate) is the
//! intended home for this module, and `AGENTS.md`'s component topology
//! (`core/hypervisor`: "Headless microkernel host & execution loop" vs.
//! `crates/api`/`crates/studio_hud`: "Presentation layer") for why an
//! HTTP/SSE/JSON-RPC protocol server doesn't belong fused into the kernel
//! crate in the first place.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// One recently-completed sovereign execution, as `get_results`/`hive_summary`
/// need it. Deliberately flat and serde-friendly — no lock guards, no
/// hypervisor-internal enum types — so it can live in a Federation-agnostic
/// crate unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSummary {
    pub sovereign: String,
    pub status: ExecStatus,
    pub domain: &'static str,
    pub output: String,
    pub duration_ms: u64,
}

/// Mirrors `federation::specialist::ExecutionStatus`'s success/failure shape
/// without naming that hypervisor-internal type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecStatus {
    Success,
    PartialSuccess,
    Failed,
    Timeout,
}

impl ExecStatus {
    /// The emoji `tool_hive_summary` prefixes each entry with.
    pub fn emoji(&self) -> &'static str {
        match self {
            ExecStatus::Success => "✅",
            ExecStatus::Failed => "❌",
            ExecStatus::PartialSuccess | ExecStatus::Timeout => "⏳",
        }
    }
}

/// One dynamic sovereign's current standing, as `get_specialists` reports it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistSummary {
    pub name: String,
    pub domain: String,
    pub confidence: f32,
    pub success_rate_pct: f32,
    pub executions: u32,
    pub has_llm: bool,
    pub has_model: bool,
    pub memory_count: usize,
    pub persona_archetype: String,
    pub model_file_name: Option<String>,
}

/// Metabolic heartbeat outcome, as `metabolic_heartbeat` reports it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatReport {
    pub token_consumed: bool,
    pub expression_rate: f32,
    pub global_tokens: f32,
    pub throttle_state: String,
}

/// Result of a `memory_sync` pull/list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySyncResult {
    pub shard: String,
    pub memories: Vec<serde_json::Value>,
    pub count: usize,
    pub total_federation_memories: usize,
}

/// Everything `McpService` needs from the running sovereign hive.
///
/// A `None`/empty return generally means "no backend attached" (the MCP
/// server can run standalone, e.g. under test, with every tool reporting
/// unavailable rather than the protocol layer itself failing) — see each
/// method's doc for its exact contract, since a few (`hive_summary_entries`,
/// `memory_pull`) also use `None`/empty to distinguish "not attached" from
/// "attached but nothing found yet", which the caller renders as different
/// user-facing messages.
#[async_trait]
pub trait IntentBackend: Send + Sync {
    /// Ask `sovereign_name` (in `domain`) to answer `input`, preferring a
    /// live specialist's own LLM and falling back to submitting a hive
    /// intent and polling for its result (bounded to ~3s). `tool_name` is
    /// recorded on the submitted intent's context for traceability.
    ///
    /// Returns `None` when no live route produced an answer in time (the
    /// caller falls back to a mock LLM response), not on any I/O error —
    /// this mirrors a "best-effort, always-answer" tool contract rather
    /// than a fallible one.
    async fn ask_sovereign(
        &self,
        sovereign_name: &str,
        domain: &str,
        input: &str,
        tool_name: &str,
    ) -> Option<String>;

    /// Internalize `name` as a first-class dynamic specialist if it isn't
    /// already registered (idempotent). `domain_hint` seeds its domain
    /// label. Returns `true` if newly registered, `false` if it already
    /// existed.
    async fn register_shard(&self, name: &str, domain_hint: &str) -> bool;

    /// The most recent `limit` execution results, newest first.
    async fn recent_results(&self, limit: usize) -> Vec<ResultSummary>;

    /// Every dynamic specialist's current standing.
    async fn specialist_roster(&self) -> Vec<SpecialistSummary>;

    /// Submit `content` as a new hive intent, wait briefly for early
    /// responders, and report how many results are queued afterward.
    async fn submit_intent(&self, content: &str) -> usize;

    /// The most recent `max_results` execution results for `hive_summary`'s
    /// markdown rendering. `None` means no backend attached; `Some(vec![])`
    /// means attached but nothing has executed yet — the two render
    /// different messages at the call site.
    async fn hive_summary_entries(&self, max_results: usize) -> Option<Vec<ResultSummary>>;

    /// Record a metabolic heartbeat from `name`, optionally consuming a
    /// token, and report the resulting system health snapshot.
    async fn heartbeat(&self, name: &str, consume_token: bool) -> HeartbeatReport;

    /// Broadcast `signal_type`/`payload` to the hive as a signal intent.
    async fn broadcast_signal(&self, signal_type: &str, payload: &serde_json::Value) -> bool;

    /// Push `entries` into `shard_name`'s memory, returning how many were
    /// successfully parsed and recorded.
    async fn memory_push(&self, shard_name: &str, entries: &[serde_json::Value]) -> usize;

    /// Pull (or list, with `include_cross` peeking at other shards' memory
    /// too) `shard_name`'s memory. `None` if the shard isn't registered.
    async fn memory_pull(&self, shard_name: &str, include_cross: bool) -> Option<MemorySyncResult>;

    /// Dispatch `instruction` (tagged with `task_id`) to the hive as a
    /// high-priority federated intent.
    async fn federated_dispatch(&self, task_id: &str, instruction: &str);
}
