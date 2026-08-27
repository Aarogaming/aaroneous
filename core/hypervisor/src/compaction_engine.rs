// Compaction Engine Pattern
// Monitors WASM agent slab utilization and triggers resurrection
// when memory fragmentation becomes critical.
//
// Snapshots agent state, kills the fragmented instance, and spawns
// a fresh, defragmented clone in milliseconds.

use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, mpsc};

use nervous_system::slab_allocator::SlabStats;

#[cfg(test)]
use nervous_system::slab_allocator::SlabAllocator;

/// Compaction Engine configuration
#[derive(Debug, Clone)]
pub struct CompactionEngineConfig {
    /// Utilization threshold to trigger reaping (0.0 to 1.0)
    pub utilization_threshold: f32,
    /// How often to check slab utilization
    pub check_interval: Duration,
    /// Maximum number of reaps before alerting
    pub max_reaps_before_alert: u32,
    /// Timeout for state snapshot/restore
    pub snapshot_timeout: Duration,
}

impl Default for CompactionEngineConfig {
    fn default() -> Self {
        Self {
            utilization_threshold: 0.8, // 80%
            check_interval: Duration::from_secs(5),
            max_reaps_before_alert: 10,
            snapshot_timeout: Duration::from_millis(500),
        }
    }
}

/// Agent state snapshot for resurrection
#[derive(Debug, Clone)]
pub struct AgentSnapshot {
    pub agent_id: String,
    pub generation: u64,
    pub timestamp: Instant,
    pub state_bytes: Vec<u8>,
    pub slab_stats: SlabStats,
    pub reap_reason: String,
}

/// Compaction event types
#[derive(Debug, Clone)]
pub enum CompactionEvent {
    /// Slab utilization exceeded threshold
    UtilizationHigh {
        agent_id: String,
        utilization: f32,
        threshold: f32,
    },
    /// Agent was reaped and resurrected
    AgentReaped {
        agent_id: String,
        old_generation: u64,
        new_generation: u64,
        reason: String,
    },
    /// Reap failed
    ReapFailed { agent_id: String, error: String },
    /// Slab recovered without reaping (bulk free)
    SlabRecovered { agent_id: String, freed_slots: u16 },
}

/// Agent handle for the Compaction Engine to manage
pub trait AgentHandle: Send + Sync {
    /// Get the agent's ID
    fn agent_id(&self) -> &str;

    /// Get current slab stats
    fn slab_stats(&self) -> SlabStats;

    /// Snapshot the agent's core state
    fn snapshot_state(&self) -> Result<Vec<u8>>;

    /// Restore state into a fresh instance
    fn restore_state(&mut self, state_bytes: &[u8]) -> Result<()>;

    /// Kill the current instance
    fn kill(&mut self);

    /// Spawn a fresh instance
    fn spawn_fresh(&mut self) -> Result<()>;

    /// Check if the agent is alive
    fn is_alive(&self) -> bool;
}

/// The Compaction Engine - monitors and reaps fragmented WASM agents
pub struct CompactionEngine {
    config: CompactionEngineConfig,
    event_tx: mpsc::UnboundedSender<CompactionEvent>,
    reaped_agents: std::collections::HashMap<String, u32>,
    running: bool,
}

impl CompactionEngine {
    pub fn new(config: CompactionEngineConfig) -> (Self, mpsc::UnboundedReceiver<CompactionEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let reaper = Self {
            config,
            event_tx,
            reaped_agents: std::collections::HashMap::new(),
            running: false,
        };
        (reaper, event_rx)
    }

    /// Check if an agent should be reaped
    pub fn should_reap(&self, stats: &SlabStats) -> bool {
        stats.utilization > self.config.utilization_threshold
    }

    /// Attempt to reap and resurrect an agent
    pub async fn reap_agent(&mut self, agent: &mut dyn AgentHandle) -> Result<AgentSnapshot> {
        let agent_id = agent.agent_id().to_string();
        let old_stats = agent.slab_stats();

        tracing::warn!(
            agent_id,
            utilization = old_stats.utilization,
            "Compaction Engine: Reaping fragmented agent"
        );

        // 1. Snapshot core state
        let state_bytes = tokio::time::timeout(self.config.snapshot_timeout, async {
            agent.snapshot_state()
        })
        .await
        .context("Snapshot timed out")??;

        let snapshot = AgentSnapshot {
            agent_id: agent_id.clone(),
            generation: old_stats.current_generation,
            timestamp: Instant::now(),
            state_bytes: state_bytes.clone(),
            slab_stats: old_stats,
            reap_reason: format!(
                "Utilization {:.1}% exceeded threshold {:.1}%",
                old_stats.utilization * 100.0,
                self.config.utilization_threshold * 100.0
            ),
        };

        // 2. Kill the fragmented instance
        agent.kill();

        // 3. Spawn fresh clone
        agent.spawn_fresh()?;

        // 4. Restore state
        agent.restore_state(&state_bytes)?;

        let new_stats = agent.slab_stats();

        // Track reaps
        let count = self.reaped_agents.entry(agent_id.clone()).or_insert(0);
        *count += 1;

        // Emit event
        let _ = self.event_tx.send(CompactionEvent::AgentReaped {
            agent_id: agent_id.clone(),
            old_generation: old_stats.current_generation,
            new_generation: new_stats.current_generation,
            reason: snapshot.reap_reason.clone(),
        });

        // Alert if excessive reaping
        if *count >= self.config.max_reaps_before_alert {
            tracing::error!(
                agent_id = agent_id,
                reap_count = *count,
                "Compaction Engine: Agent reaped excessively - possible memory leak"
            );
        }

        tracing::info!(
            agent_id = agent_id,
            old_utilization = old_stats.utilization,
            new_utilization = new_stats.utilization,
            "Compaction Engine: Agent resurrected with clean memory"
        );

        Ok(snapshot)
    }

    /// Attempt bulk free of committed slots (less aggressive than full reap)
    pub fn try_bulk_free(&mut self, agent: &mut dyn AgentHandle) -> u16 {
        let stats = agent.slab_stats();
        if stats.committed_count > 0 {
            // The agent handle should implement bulk free
            // This is a hint that committed slots can be reclaimed
            let _ = self.event_tx.send(CompactionEvent::SlabRecovered {
                agent_id: agent.agent_id().to_string(),
                freed_slots: stats.committed_count,
            });
            return stats.committed_count;
        }
        0
    }

    /// Run the reaper monitoring loop
    pub async fn run_monitor(&mut self, mut agents: Vec<Box<dyn AgentHandle>>) -> Result<()> {
        self.running = true;
        let mut interval = tokio::time::interval(self.config.check_interval);

        while self.running {
            interval.tick().await;

            for agent in &mut agents {
                if !agent.is_alive() {
                    continue;
                }

                let stats = agent.slab_stats();

                // Check if utilization is high
                if stats.utilization > self.config.utilization_threshold {
                    let _ = self.event_tx.send(CompactionEvent::UtilizationHigh {
                        agent_id: agent.agent_id().to_string(),
                        utilization: stats.utilization,
                        threshold: self.config.utilization_threshold,
                    });

                    // Try bulk free first
                    let freed = self.try_bulk_free(agent.as_mut());
                    let new_stats = agent.slab_stats();

                    // If still high after bulk free, full reap
                    if new_stats.utilization > self.config.utilization_threshold {
                        if let Err(e) = self.reap_agent(agent.as_mut()).await {
                            let _ = self.event_tx.send(CompactionEvent::ReapFailed {
                                agent_id: agent.agent_id().to_string(),
                                error: e.to_string(),
                            });
                        }
                    } else if freed > 0 {
                        tracing::info!(
                            agent_id = agent.agent_id(),
                            freed,
                            "Compaction Engine: Bulk free recovered slab"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Stop the monitoring loop
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Get reap statistics
    pub fn reap_stats(&self) -> ReaperStats {
        ReaperStats {
            total_reaps: self.reaped_agents.values().sum(),
            agents_reaped: self.reaped_agents.len(),
            most_reaped: self
                .reaped_agents
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(id, count)| (id.clone(), *count)),
        }
    }
}

/// Compaction statistics summary
#[derive(Debug, Clone)]
pub struct ReaperStats {
    pub total_reaps: u32,
    pub agents_reaped: usize,
    pub most_reaped: Option<(String, u32)>,
}

/// Shared Compaction Engine for multi-agent monitoring
pub type SharedCompactionEngine = Arc<RwLock<CompactionEngine>>;

/// Builder for Compaction Engine configuration
pub struct CompactionEngineBuilder {
    config: CompactionEngineConfig,
}

impl CompactionEngineBuilder {
    pub fn new() -> Self {
        Self {
            config: CompactionEngineConfig::default(),
        }
    }

    pub fn utilization_threshold(mut self, threshold: f32) -> Self {
        self.config.utilization_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn check_interval(mut self, interval: Duration) -> Self {
        self.config.check_interval = interval;
        self
    }

    pub fn max_reaps_before_alert(mut self, max: u32) -> Self {
        self.config.max_reaps_before_alert = max;
        self
    }

    pub fn snapshot_timeout(mut self, timeout: Duration) -> Self {
        self.config.snapshot_timeout = timeout;
        self
    }

    pub fn build(self) -> (CompactionEngine, mpsc::UnboundedReceiver<CompactionEvent>) {
        CompactionEngine::new(self.config)
    }
}

impl Default for CompactionEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockAgent {
        id: String,
        alive: bool,
        slab: Mutex<SlabAllocator>,
        state: Mutex<Vec<u8>>,
    }

    impl MockAgent {
        fn new(id: &str, capacity: usize) -> Self {
            Self {
                id: id.to_string(),
                alive: true,
                slab: Mutex::new(SlabAllocator::new(capacity)),
                state: Mutex::new(vec![1, 2, 3, 4]),
            }
        }
    }

    impl AgentHandle for MockAgent {
        fn agent_id(&self) -> &str {
            &self.id
        }

        fn slab_stats(&self) -> SlabStats {
            self.slab.lock().unwrap().stats()
        }

        fn snapshot_state(&self) -> Result<Vec<u8>> {
            Ok(self.state.lock().unwrap().clone())
        }

        fn restore_state(&mut self, state_bytes: &[u8]) -> Result<()> {
            *self.state.lock().unwrap() = state_bytes.to_vec();
            Ok(())
        }

        fn kill(&mut self) {
            self.alive = false;
            self.slab.lock().unwrap().reset();
        }

        fn spawn_fresh(&mut self) -> Result<()> {
            self.alive = true;
            Ok(())
        }

        fn is_alive(&self) -> bool {
            self.alive
        }
    }

    #[tokio::test]
    async fn test_compaction_engine_should_reap() {
        let (reaper, _rx) = CompactionEngine::new(CompactionEngineConfig {
            utilization_threshold: 0.5,
            ..Default::default()
        });

        let mut slab = SlabAllocator::new(10);
        for _ in 0..6 {
            slab.allocate(0, 1, 1).unwrap();
        }

        assert!(reaper.should_reap(&slab.stats()));
    }

    #[tokio::test]
    async fn test_compaction_engine_reap_agent() {
        let (mut reaper, _rx) = CompactionEngine::new(CompactionEngineConfig {
            utilization_threshold: 0.5,
            ..Default::default()
        });

        let mut agent = MockAgent::new("test_agent", 10);
        // Fill slab to trigger reaping
        for _ in 0..6 {
            agent.slab.lock().unwrap().allocate(0, 1, 1).unwrap();
        }

        let snapshot = reaper.reap_agent(&mut agent).await.unwrap();
        assert_eq!(snapshot.agent_id, "test_agent");
        assert!(agent.is_alive());
        assert_eq!(agent.slab.lock().unwrap().free_count(), 10); // Clean slab
    }
}
