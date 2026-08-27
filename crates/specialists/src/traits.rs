//! traits.rs
//! Standard Machine-Native contracts for the Federated Sovereign Specialists and Relic Engines.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Standard Machine-Native Linguistic Protocol (MNLP) Packet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MnlpPacket {
    pub opcode: u16,
    pub source: String,
    pub target: String,
    pub correlation_id: u64,
    pub payload: Vec<u8>,
}

/// Standard MNLP Response payload returned after execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MnlpResponse {
    pub success: bool,
    pub opcode: u16,
    pub correlation_id: u64,
    pub message: String,
    pub payload: Vec<u8>,
}

/// Operational Health and Metabolic status of a Sovereign Specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistHealth {
    pub name: String,
    pub domain_opcode: u16,
    pub tokens: f32,
    pub max_tokens: f32,
    pub backlog_count: usize,
    pub is_dormant: bool,
    pub last_active: DateTime<Utc>,
}

/// Standard trait contract implemented by every Federated Sovereign Specialist
#[async_trait]
pub trait SovereignSpecialist: Send + Sync {
    /// Canonical specialist name (e.g. "Orchestrator", "Synthesizer")
    fn name(&self) -> &'static str;

    /// Primary Machine-Native domain opcode (e.g. 0x0100 for Task Orchestration)
    fn domain_opcode(&self) -> u16;

    /// Process an incoming machine-native packet
    async fn handle_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse>;

    /// Ingest metabolic tokens to sustain execution
    fn recharge_metabolism(&mut self, tokens: f32);

    /// Current operational and metabolic health status
    fn health_report(&self) -> SpecialistHealth;

    /// Exports a serializable state representation for Compaction Engine hibernation
    fn hibernate_state(&self) -> Result<Vec<u8>> {
        let health = self.health_report();
        serde_json::to_vec(&health).map_err(Into::into)
    }

    /// Reconstitutes specialist state from a hibernation snapshot
    fn resurrect_state(&mut self, _snapshot: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// Standard trait contract implemented by every autonomous Relic Engine
pub trait RelicEngine: Send + Sync {
    /// Canonical name of the Relic Engine (e.g. "OrchestratorCore", "KnowledgeStore", "MemoryIndex")
    fn relic_name(&self) -> &'static str;

    /// The name of the supervising federated specialist
    fn supervisor_name(&self) -> &'static str;

    /// Current operational metrics of the relic engine
    fn relic_status(&self) -> String;
}
