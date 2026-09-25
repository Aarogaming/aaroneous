//! `impl IntentBackend for Federation` — the concrete adapter wiring the MCP
//! protocol layer (`crate::mcp_service`) to the live sovereign hive.
//!
//! This is the one place that reaches into `Federation`'s fields
//! (`dynamic`, `results`, `biology`, `submit_intent`) on `mcp_service`'s
//! behalf; `service.rs` itself only ever sees the `IntentBackend` trait.
//! Every method here is a direct, behavior-preserving relocation of logic
//! that used to live inline in `mcp_service::service`'s `self.federation`
//! branches — see `docs/TECH_DEBT_TEST_DUPLICATION.md` §2.2 for why.

use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use super::Federation;
use crate::federation::specialist::ExecutionStatus;
use crate::federation::specialists::GenericSpecialist;
use mcp_server::mcp_service::{
    ExecStatus, HeartbeatReport, IntentBackend, MemorySyncResult, ResultSummary, SpecialistSummary,
};

impl From<ExecutionStatus> for ExecStatus {
    fn from(s: ExecutionStatus) -> Self {
        match s {
            ExecutionStatus::Success => ExecStatus::Success,
            ExecutionStatus::PartialSuccess => ExecStatus::PartialSuccess,
            ExecutionStatus::Failed => ExecStatus::Failed,
            ExecutionStatus::Timeout => ExecStatus::Timeout,
        }
    }
}

#[async_trait]
impl IntentBackend for Federation {
    async fn ask_sovereign(
        &self,
        sovereign_name: &str,
        domain: &str,
        input: &str,
        tool_name: &str,
    ) -> Option<String> {
        use crate::federation::specialists::system_prompt_for_domain;

        // Prefer a live dynamic specialist's own LLM.
        {
            let dynamic = self.dynamic.read().await;
            if let Some(s) = dynamic.iter().find(|s| s.name == sovereign_name)
                && let Some(ref llm) = s.llm
            {
                let system_prompt = system_prompt_for_domain(domain, sovereign_name);
                return Some(
                    llm.generate_domain_response(&system_prompt, input, domain)
                        .await
                        .unwrap_or_else(|e| format!("[{}] LLM error: {}", sovereign_name, e)),
                );
            }
        }

        // Fallback: submit as a hive intent and poll for its result (max ~3s).
        let count_before = self.results.lock().await.len();
        let mut intent = crate::federation::intent::Intent::new(input.to_string());
        intent
            .context
            .insert("target_sovereign".to_string(), sovereign_name.to_string());
        intent
            .context
            .insert("mcp_tool".to_string(), tool_name.to_string());
        self.submit_intent(intent).await;

        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(3000);
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            let results = self.results.lock().await;
            let new: Vec<_> = results.iter().skip(count_before).collect();
            if let Some(r) = new.iter().find(|r| {
                r.specialist_name.as_deref() == Some(sovereign_name)
                    || r.specialist.sovereign_name() == sovereign_name
            }) {
                return Some(r.output.clone());
            }
            if !new.is_empty()
                && tokio::time::Instant::now() >= deadline - tokio::time::Duration::from_millis(200)
            {
                return new.last().map(|r| r.output.clone());
            }
            drop(results);
            if tokio::time::Instant::now() >= deadline {
                return None;
            }
        }
    }

    async fn register_shard(&self, name: &str, domain_hint: &str) -> bool {
        let mut dynamic = self.dynamic.write().await;
        if dynamic.iter().any(|s| s.name == name) {
            return false;
        }
        let specialist = GenericSpecialist::new(name, domain_hint);
        dynamic.push(Arc::new(specialist));
        drop(dynamic);

        let mut biology = self.biology.write().await;
        biology.register_specialist(name, 5000); // 5s default heartbeat
        info!("Internalized AAS Shard: {}", name);
        true
    }

    async fn recent_results(&self, limit: usize) -> Vec<ResultSummary> {
        let results = self.results.lock().await;
        results
            .iter()
            .rev()
            .take(limit)
            .map(|r| ResultSummary {
                sovereign: r
                    .specialist_name
                    .clone()
                    .unwrap_or_else(|| r.specialist.name().to_string()),
                status: r.status.into(),
                domain: r.specialist.domain(),
                output: r.output.chars().take(500).collect(),
                duration_ms: r.duration_ms,
            })
            .collect()
    }

    async fn specialist_roster(&self) -> Vec<SpecialistSummary> {
        let dynamic = self.dynamic.read().await;
        dynamic
            .iter()
            .map(|s| {
                let l = s.learning.lock();
                let success_rate = if l.total_executions > 0 {
                    l.success_count as f32 / l.total_executions as f32 * 100.0
                } else {
                    0.0
                };
                let memory_count = s.memory.lock().count_for(&s.name);
                let persona_archetype = s
                    .persona
                    .as_ref()
                    .map(|p| p.personality_persona.archetype.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                SpecialistSummary {
                    name: s.name.clone(),
                    domain: s.domain.clone(),
                    confidence: (l.confidence_score * 100.0).round() / 100.0,
                    success_rate_pct: (success_rate * 10.0).round() / 10.0,
                    executions: l.total_executions,
                    has_llm: s.llm.is_some(),
                    has_model: s.model_path.is_some(),
                    memory_count,
                    persona_archetype,
                    model_file_name: s
                        .model_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string()),
                }
            })
            .collect()
    }

    async fn submit_intent(&self, content: &str) -> usize {
        let intent = crate::federation::intent::Intent::new(content.to_string());
        Federation::submit_intent(self, intent).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        self.results.lock().await.len()
    }

    async fn hive_summary_entries(&self, max_results: usize) -> Option<Vec<ResultSummary>> {
        let results = self.results.lock().await;
        Some(
            results
                .iter()
                .rev()
                .take(max_results)
                .map(|r| ResultSummary {
                    sovereign: r
                        .specialist_name
                        .clone()
                        .unwrap_or_else(|| r.specialist.sovereign_name().to_string()),
                    status: r.status.into(),
                    domain: r.specialist.domain(),
                    // hive_summary renders the full (JSON-formatted where
                    // possible) output, not the 500-char excerpt
                    // recent_results uses — no truncation here.
                    output: r.output.clone(),
                    duration_ms: r.duration_ms,
                })
                .collect(),
        )
    }

    async fn heartbeat(&self, name: &str, consume_token: bool) -> HeartbeatReport {
        let mut biology = self.biology.write().await;
        let token_consumed = if consume_token {
            biology.consume_specialist_token(name)
        } else {
            true
        };
        let report = biology.get_health_report();
        HeartbeatReport {
            token_consumed,
            expression_rate: report.expression_rate,
            global_tokens: report.global_tokens,
            throttle_state: report.throttle_state.to_string(),
        }
    }

    async fn broadcast_signal(&self, signal_type: &str, payload: &serde_json::Value) -> bool {
        let intent_content = format!("SIGNAL: {} | PAYLOAD: {}", signal_type, payload);
        let intent = crate::federation::intent::Intent::new(intent_content);
        Federation::submit_intent(self, intent).await;
        true
    }

    async fn memory_push(&self, shard_name: &str, entries: &[serde_json::Value]) -> usize {
        let mut count = 0;
        for entry_json in entries {
            if let Ok(entry) =
                serde_json::from_value::<crate::specialist_memory::MemoryEntry>(entry_json.clone())
            {
                let dynamic = self.dynamic.read().await;
                if let Some(spec) = dynamic.iter().find(|s| s.name == shard_name) {
                    spec.memory.lock().record_memory(entry);
                    count += 1;
                }
            }
        }
        count
    }

    async fn memory_pull(&self, shard_name: &str, include_cross: bool) -> Option<MemorySyncResult> {
        let dynamic = self.dynamic.read().await;
        let spec = dynamic.iter().find(|s| s.name == shard_name)?;
        let memory = spec.memory.lock();
        let all_memories = memory.memories();

        let mut response_memories = Vec::new();
        if let Some(local) = all_memories.get(shard_name) {
            response_memories.extend(local.iter().cloned());
        }
        if include_cross {
            for (other_shard, memories) in all_memories {
                if other_shard != shard_name {
                    response_memories.extend(memories.iter().take(2).cloned());
                }
            }
        }

        let count = response_memories.len();
        let total_federation_memories = memory.total_count();
        Some(MemorySyncResult {
            shard: shard_name.to_string(),
            memories: response_memories
                .into_iter()
                .filter_map(|m| serde_json::to_value(m).ok())
                .collect(),
            count,
            total_federation_memories,
        })
    }

    async fn federated_dispatch(&self, task_id: &str, instruction: &str) {
        let mut intent = crate::federation::intent::Intent::new(instruction.to_string());
        intent
            .context
            .insert("task_id".to_string(), task_id.to_string());
        intent
            .context
            .insert("source".to_string(), "Guild_Federation".to_string());
        Federation::submit_intent(self, intent).await;
    }
}
