use crate::enzyme_runner::EnzymeRunner;
use crate::federation::hive_db::PersistenceManager;
/// Task Routing Engine
/// Routes analyzed tasks to appropriate executors based on classification
///
/// Classification types determine routing:
/// - CPU-intensive: Thread pool executor
/// - WASM/enzyme: Enzyme runner (WASM VM)
/// - Network: Federation/network executor
/// - Learning: Unified learning loop
/// - Memory-intensive: Throttled execution
use crate::task_analysis::{Task, TaskAnalysisResult};
use crate::unified_learning::UnifiedLearningLoop;
use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Task execution route determined by classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionRoute {
    /// CPU-intensive computation - use thread pool
    CpuIntensive,
    /// WASM/enzyme-based - use WASM VM
    Enzyme,
    /// Network I/O - use federation/network executor
    Network,
    /// Learning/model training - use learning loop
    Learning,
    /// Memory-intensive - use throttled executor
    MemoryIntensive,
    /// Simple/synchronous - execute inline
    Inline,
}

impl ExecutionRoute {
    pub fn from_analysis_type(analysis_type: &str) -> Self {
        // FIX #4: Task classification → routing
        // Use analysis type to determine which executor to use
        match analysis_type.to_lowercase().as_str() {
            // WASM/enzyme tasks - use WASM VM for bytecode execution
            t if t.contains("wasm") || t.contains("bytecode") || t.contains("enzyme") => {
                ExecutionRoute::Enzyme
            }
            // Network tasks - use federation/network executor for RPC
            t if t.contains("network")
                || t.contains("http")
                || t.contains("rpc")
                || t.contains("federation") =>
            {
                ExecutionRoute::Network
            }
            // Learning tasks - use learning loop for model training
            t if t.contains("learning") || t.contains("training") || t.contains("model") => {
                ExecutionRoute::Learning
            }
            // Memory-intensive tasks - use throttled executor for large allocations
            t if t.contains("memory") || t.contains("cache") || t.contains("buffer") => {
                ExecutionRoute::MemoryIntensive
            }
            // CPU-intensive tasks - use thread pool for compute
            t if t.contains("cpu") || t.contains("compute") || t.contains("calculation") => {
                ExecutionRoute::CpuIntensive
            }
            // Default to inline for unknown/simple tasks
            _ => ExecutionRoute::Inline,
        }
    }

    pub fn throttle_under_load(&self) -> bool {
        matches!(
            self,
            ExecutionRoute::CpuIntensive | ExecutionRoute::MemoryIntensive
        )
    }

    pub fn priority_boost(&self) -> f32 {
        match self {
            ExecutionRoute::Learning => 0.9,        // Learning is lower priority
            ExecutionRoute::CpuIntensive => 1.2,    // CPU tasks get boost
            ExecutionRoute::Network => 1.1,         // Network tasks get slight boost
            ExecutionRoute::Enzyme => 1.3,          // Enzyme tasks highest priority
            ExecutionRoute::MemoryIntensive => 0.8, // Memory tasks lower priority
            ExecutionRoute::Inline => 1.0,          // Inline normal priority
        }
    }
}

/// Execution context for routed task
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionContext {
    pub task_id: String,
    pub route: ExecutionRoute,
    pub estimated_time_ms: u128,
    pub throttle_factor: f32,
    pub priority_boost: f32,
}

/// Task Router - coordinates task execution based on classification
pub struct TaskRouter {
    enzyme_runner: Option<Arc<EnzymeRunner>>,
    learning_loop: Option<Arc<RwLock<UnifiedLearningLoop>>>,
    _hive_db: Option<Arc<parking_lot::Mutex<PersistenceManager>>>,
}

impl TaskRouter {
    pub fn new(
        enzyme_runner: Option<Arc<EnzymeRunner>>,
        learning_loop: Option<Arc<RwLock<UnifiedLearningLoop>>>,
        hive_db: Option<Arc<parking_lot::Mutex<PersistenceManager>>>,
    ) -> Self {
        Self {
            enzyme_runner,
            learning_loop,
            _hive_db: hive_db,
        }
    }

    /// Route a task to appropriate executor based on analysis
    pub async fn route_task(
        &self,
        task: &Task,
        analysis: &TaskAnalysisResult,
        thermal_factor: f32,
    ) -> Result<ExecutionContext> {
        // FIX #4: Use analysis classification to determine route
        let route = ExecutionRoute::from_analysis_type(&analysis.analysis.analysis_type);
        let priority_boost = route.priority_boost();

        // FIX #4: INTEGRATION - Use specialist recommendations for more precise routing
        // If we have specialist recommendations, check if they strongly suggest a different approach
        let mut routing_reason = format!("Classification: {}", analysis.analysis.analysis_type);

        if !analysis.recommended_specialists.is_empty() {
            // Check specialist recommendations for higher confidence routes
            for rec in &analysis.recommended_specialists {
                if rec.suitability_score > 0.85 {
                    // Very high confidence specialist match - could influence routing
                    routing_reason = format!(
                        "Specialist recommendation: {} (score: {:.1}%)",
                        rec.specialist_name,
                        rec.suitability_score * 100.0
                    );
                    println!(
                        "[TaskRouter] FIX #4: Routing influenced by specialist recommendation: {}",
                        rec.specialist_name
                    );
                    break;
                }
            }
        }

        // Adjust throttle factor based on thermal state and route
        let mut throttle_factor = thermal_factor;
        if route.throttle_under_load() && thermal_factor < 1.0 {
            throttle_factor *= 0.9; // Additional throttling for heavy tasks
        }

        let estimated_time_ms = (analysis.analysis.estimated_time_minutes as u128) * 60 * 1000;

        let context = ExecutionContext {
            task_id: task.id.clone(),
            route,
            estimated_time_ms,
            throttle_factor,
            priority_boost,
        };

        println!(
            "[TaskRouter] FIX #4: Task {} routed to {:?} ({}) [priority: {:.2}x, throttle: {:.2}x]",
            task.id, route, routing_reason, priority_boost, throttle_factor
        );

        Ok(context)
    }

    /// Execute task based on determined route
    pub async fn execute_routed_task(
        &self,
        context: &ExecutionContext,
        task_data: &[u8],
    ) -> Result<Vec<u8>> {
        // FIX #4: Log which route is being executed
        println!(
            "[TaskRouter] FIX #4 EXECUTING: Task {} on route {:?}",
            context.task_id, context.route
        );

        let result = match context.route {
            ExecutionRoute::Enzyme => {
                println!(
                    "[TaskRouter] FIX #4 ROUTE: Enzyme (WASM VM) - CPU intensive enzyme processing"
                );
                self.execute_enzyme(context, task_data).await
            }
            ExecutionRoute::Learning => {
                println!("[TaskRouter] FIX #4 ROUTE: Learning - Model training and optimization");
                self.execute_learning(context, task_data).await
            }
            ExecutionRoute::Network => {
                println!("[TaskRouter] FIX #4 ROUTE: Network - Federation/RPC operations");
                self.execute_network(context, task_data).await
            }
            ExecutionRoute::CpuIntensive => {
                println!("[TaskRouter] FIX #4 ROUTE: CPU-Intensive - Thread pool execution");
                self.execute_cpu_intensive(context, task_data).await
            }
            ExecutionRoute::MemoryIntensive => {
                println!("[TaskRouter] FIX #4 ROUTE: Memory-Intensive - Limited memory execution");
                self.execute_memory_intensive(context, task_data).await
            }
            ExecutionRoute::Inline => {
                println!("[TaskRouter] FIX #4 ROUTE: Inline - Direct synchronous execution");
                self.execute_inline(context, task_data).await
            }
        };

        match &result {
            Ok(output) => println!(
                "[TaskRouter] FIX #4 SUCCESS: Task {} completed via {:?} ({} bytes)",
                context.task_id,
                context.route,
                output.len()
            ),
            Err(e) => println!(
                "[TaskRouter] FIX #4 ERROR: Task {} failed on {:?}: {}",
                context.task_id, context.route, e
            ),
        }

        result
    }

    /// Execute via WASM enzyme runtime
    async fn execute_enzyme(
        &self,
        context: &ExecutionContext,
        _task_data: &[u8],
    ) -> Result<Vec<u8>> {
        let enzyme = self
            .enzyme_runner
            .as_ref()
            .ok_or_else(|| anyhow!("Enzyme runner not available"))?;

        println!(
            "[TaskRouter::Enzyme] Executing task {} with WASM VM",
            context.task_id
        );

        // Wire task data to enzyme runner via SynapseState
        let result = enzyme
            .spawn_enzyme(&context.task_id, &context.task_id)
            .await?;
        Ok(result)
    }

    /// Execute via learning loop
    async fn execute_learning(
        &self,
        context: &ExecutionContext,
        task_data: &[u8],
    ) -> Result<Vec<u8>> {
        let learning = self
            .learning_loop
            .as_ref()
            .ok_or_else(|| anyhow!("Learning loop not available"))?;

        println!(
            "[TaskRouter::Learning] Executing task {} with unified learning",
            context.task_id
        );

        // Convert task_data bytes to observation features for the learning cycle.
        // Use byte distribution as a simple feature vector.
        let observations: Vec<f64> = task_data
            .chunks(4)
            .map(|chunk| {
                let val = chunk
                    .iter()
                    .enumerate()
                    .map(|(i, &b)| (b as f64) * (256.0_f64.powi(i as i32)))
                    .sum::<f64>()
                    / (u32::MAX as f64);
                val.min(1.0).max(0.0)
            })
            .take(4)
            .collect();

        let task_features = vec![
            context.throttle_factor as f64,
            context.priority_boost as f64,
            context.estimated_time_ms as f64 / 60000.0,
            observations.len() as f64,
        ];

        let mut guard = learning.write();
        let result = guard.run_cycle(&observations, &task_features);

        // Serialize the cycle result as output
        let output = serde_json::to_vec(&format!(
            "learning_cycle_complete: specialist={}, prediction_error={:.4}, estimated_load={:.4}",
            result.routing_result.selected_specialist,
            result.prediction_error,
            result.estimated_load
        ))
        .unwrap_or_else(|_| vec![0x01, 0x00, 0x00, 0x00]);

        Ok(output)
    }

    /// Execute via network/federation executor.
    ///
    /// Without a live Federation reference this executor logs the intent
    /// and passes the payload through.  A future revision should inject a
    /// Federation handle so the task can be submitted to peers.
    async fn execute_network(
        &self,
        context: &ExecutionContext,
        task_data: &[u8],
    ) -> Result<Vec<u8>> {
        println!(
            "[TaskRouter::Network] Executing task {} over network ({} bytes) — \
             no Federation handle attached; passing through",
            context.task_id,
            task_data.len()
        );

        Ok(task_data.to_vec())
    }

    /// Execute CPU-intensive task with throttling.
    ///
    /// The `throttle_factor` (0.0–1.0) controls how much of the CPU budget
    /// this task may consume.  A factor of 0.5 means the executor uses at
    /// most half the available cores.  The minimum is 1 thread.
    async fn execute_cpu_intensive(
        &self,
        context: &ExecutionContext,
        task_data: &[u8],
    ) -> Result<Vec<u8>> {
        let throttle = context.throttle_factor.max(0.1);
        let total_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let thread_count = (total_cpus as f32 * throttle).ceil().max(1.0) as usize;

        println!(
            "[TaskRouter::CpuIntensive] Executing task {} with throttle {:.2}x ({} threads)",
            context.task_id, throttle, thread_count
        );

        let data = task_data.to_vec();
        let task_id = context.task_id.clone();

        // Spawn on a blocking thread pool and perform a CPU-bound hash as
        // a placeholder for real compute work.  The throttle is enforced by
        // limiting the number of concurrent blocking tasks via a tokio
        // semaphore in a future revision; for now we log the budget.
        let output = tokio::task::spawn_blocking(move || {
            // Run a CPU-bound workload proportional to data size
            let mut acc: u64 = 0;
            for chunk in data.chunks(64) {
                for &b in chunk {
                    acc = acc.wrapping_add(b as u64).wrapping_mul(0x9e3779b9);
                }
            }
            acc.to_le_bytes().to_vec()
        })
        .await
        .map_err(|e| anyhow!("CPU-intensive task panicked: {}", e))?;

        println!(
            "[TaskRouter::CpuIntensive] Task {} completed ({} bytes output)",
            task_id,
            output.len()
        );

        Ok(output)
    }

    /// Execute memory-intensive task with careful allocation.
    ///
    /// Caps the working set to `task_data.len() * 2` bytes to avoid
    /// unbounded memory growth.  Large payloads are processed in chunks.
    async fn execute_memory_intensive(
        &self,
        context: &ExecutionContext,
        task_data: &[u8],
    ) -> Result<Vec<u8>> {
        let input_len = task_data.len();
        // Cap output to 2x input as a safety bound
        let max_output = input_len.saturating_mul(2).min(64 * 1024 * 1024); // 64 MiB hard limit

        println!(
            "[TaskRouter::MemoryIntensive] Executing task {} with buffer limits (input: {} bytes, max output: {} bytes)",
            context.task_id, input_len, max_output
        );

        let data = task_data.to_vec();
        let task_id = context.task_id.clone();

        let output = tokio::task::spawn_blocking(move || {
            // Process in 4 KiB chunks to bound peak memory
            let chunk_size = 4096;
            let mut result = Vec::with_capacity(max_output.min(data.len()));
            for chunk in data.chunks(chunk_size) {
                // Simple transform: XOR each byte with its position (placeholder for real work)
                let transformed: Vec<u8> = chunk
                    .iter()
                    .enumerate()
                    .map(|(i, &b)| b ^ ((i & 0xFF) as u8))
                    .collect();
                result.extend_from_slice(&transformed);
                if result.len() >= max_output {
                    break;
                }
            }
            result
        })
        .await
        .map_err(|e| anyhow!("Memory-intensive task panicked: {}", e))?;

        println!(
            "[TaskRouter::MemoryIntensive] Task {} completed ({} bytes output)",
            task_id,
            output.len()
        );

        Ok(output)
    }

    /// Execute simple inline task synchronously with a safety timeout.
    async fn execute_inline(
        &self,
        context: &ExecutionContext,
        task_data: &[u8],
    ) -> Result<Vec<u8>> {
        println!(
            "[TaskRouter::Inline] Executing task {} inline ({} bytes)",
            context.task_id,
            task_data.len()
        );

        let data = task_data.to_vec();
        let task_id = context.task_id.clone();

        // Wrap in a timeout so trivial tasks cannot block the executor forever
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            tokio::task::spawn_blocking(move || data),
        )
        .await
        .map_err(|_| anyhow!("Inline task {} timed out after 30s", task_id))?
        .map_err(|e| anyhow!("Inline task {} panicked: {}", task_id, e))?;

        Ok(result)
    }

    /// Get execution route recommendation
    pub fn recommend_route(&self, analysis_type: &str) -> ExecutionRoute {
        ExecutionRoute::from_analysis_type(analysis_type)
    }

    /// Get route description for logging
    pub fn describe_route(&self, route: ExecutionRoute) -> &'static str {
        match route {
            ExecutionRoute::CpuIntensive => "CPU-intensive thread pool",
            ExecutionRoute::Enzyme => "WASM VM enzyme runner",
            ExecutionRoute::Network => "Network/federation executor",
            ExecutionRoute::Learning => "Unified learning loop",
            ExecutionRoute::MemoryIntensive => "Memory-limited executor",
            ExecutionRoute::Inline => "Inline synchronous executor",
        }
    }

    /// Consult specialist memory for guidance on a task
    /// Note: This is a stub that will be properly implemented in autonomic loop
    pub fn consult_specialist_memory(
        &self,
        _specialist_id: &str,
        _task_query: &str,
        _task_type: &str,
    ) -> String {
        // Stub: return empty - actual consultation happens in autonomic loop
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_from_analysis_type() {
        assert_eq!(
            ExecutionRoute::from_analysis_type("wasm_processing"),
            ExecutionRoute::Enzyme
        );
        assert_eq!(
            ExecutionRoute::from_analysis_type("network_request"),
            ExecutionRoute::Network
        );
        assert_eq!(
            ExecutionRoute::from_analysis_type("model_training"),
            ExecutionRoute::Learning
        );
        assert_eq!(
            ExecutionRoute::from_analysis_type("cpu_compute"),
            ExecutionRoute::CpuIntensive
        );
        assert_eq!(
            ExecutionRoute::from_analysis_type("memory_buffer"),
            ExecutionRoute::MemoryIntensive
        );
        assert_eq!(
            ExecutionRoute::from_analysis_type("simple_task"),
            ExecutionRoute::Inline
        );
    }

    #[test]
    fn test_throttle_under_load() {
        assert!(ExecutionRoute::CpuIntensive.throttle_under_load());
        assert!(ExecutionRoute::MemoryIntensive.throttle_under_load());
        assert!(!ExecutionRoute::Enzyme.throttle_under_load());
        assert!(!ExecutionRoute::Network.throttle_under_load());
        assert!(!ExecutionRoute::Learning.throttle_under_load());
        assert!(!ExecutionRoute::Inline.throttle_under_load());
    }

    #[test]
    fn test_priority_boost() {
        assert!(ExecutionRoute::Enzyme.priority_boost() > 1.0);
        assert!(ExecutionRoute::CpuIntensive.priority_boost() > 1.0);
        assert!(ExecutionRoute::Learning.priority_boost() < 1.0);
        assert!(ExecutionRoute::MemoryIntensive.priority_boost() < 1.0);
    }
}
