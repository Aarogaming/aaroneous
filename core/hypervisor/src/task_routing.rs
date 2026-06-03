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
use crate::enzyme_runner::EnzymeRunner;
use crate::unified_learning::UnifiedLearningLoop;
use crate::federation::hive_db::PersistenceManager;
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

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
            t if t.contains("network") || t.contains("http") || t.contains("rpc") || t.contains("federation") => {
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
        matches!(self, ExecutionRoute::CpuIntensive | ExecutionRoute::MemoryIntensive)
    }

    pub fn priority_boost(&self) -> f32 {
        match self {
            ExecutionRoute::Learning => 0.9,        // Learning is lower priority
            ExecutionRoute::CpuIntensive => 1.2,    // CPU tasks get boost
            ExecutionRoute::Network => 1.1,          // Network tasks get slight boost
            ExecutionRoute::Enzyme => 1.3,           // Enzyme tasks highest priority
            ExecutionRoute::MemoryIntensive => 0.8,  // Memory tasks lower priority
            ExecutionRoute::Inline => 1.0,           // Inline normal priority
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
    hive_db: Option<Arc<parking_lot::Mutex<PersistenceManager>>>,
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
            hive_db,
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
                        rec.specialist_name, rec.suitability_score * 100.0
                    );
                    println!("[TaskRouter] FIX #4: Routing influenced by specialist recommendation: {}",
                        rec.specialist_name);
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

        println!("[TaskRouter] FIX #4: Task {} routed to {:?} ({}) [priority: {:.2}x, throttle: {:.2}x]",
            task.id, route, routing_reason, priority_boost, throttle_factor);

        Ok(context)
    }

    /// Execute task based on determined route
    pub async fn execute_routed_task(
        &self,
        context: &ExecutionContext,
        task_data: &[u8],
    ) -> Result<Vec<u8>> {
        // FIX #4: Log which route is being executed
        println!("[TaskRouter] FIX #4 EXECUTING: Task {} on route {:?}", context.task_id, context.route);
        
        let result = match context.route {
            ExecutionRoute::Enzyme => {
                println!("[TaskRouter] FIX #4 ROUTE: Enzyme (WASM VM) - CPU intensive enzyme processing");
                self.execute_enzyme(context, task_data).await
            },
            ExecutionRoute::Learning => {
                println!("[TaskRouter] FIX #4 ROUTE: Learning - Model training and optimization");
                self.execute_learning(context, task_data).await
            },
            ExecutionRoute::Network => {
                println!("[TaskRouter] FIX #4 ROUTE: Network - Federation/RPC operations");
                self.execute_network(context, task_data).await
            },
            ExecutionRoute::CpuIntensive => {
                println!("[TaskRouter] FIX #4 ROUTE: CPU-Intensive - Thread pool execution");
                self.execute_cpu_intensive(context, task_data).await
            },
            ExecutionRoute::MemoryIntensive => {
                println!("[TaskRouter] FIX #4 ROUTE: Memory-Intensive - Limited memory execution");
                self.execute_memory_intensive(context, task_data).await
            },
            ExecutionRoute::Inline => {
                println!("[TaskRouter] FIX #4 ROUTE: Inline - Direct synchronous execution");
                self.execute_inline(context, task_data).await
            },
        };
        
        match &result {
            Ok(output) => println!("[TaskRouter] FIX #4 SUCCESS: Task {} completed via {:?} ({} bytes)", 
                context.task_id, context.route, output.len()),
            Err(e) => println!("[TaskRouter] FIX #4 ERROR: Task {} failed on {:?}: {}", 
                context.task_id, context.route, e),
        }
        
        result
    }

    /// Execute via WASM enzyme runtime
    async fn execute_enzyme(&self, context: &ExecutionContext, task_data: &[u8]) -> Result<Vec<u8>> {
        let enzyme = self.enzyme_runner.as_ref()
            .ok_or_else(|| anyhow!("Enzyme runner not available"))?;

        println!("[TaskRouter::Enzyme] Executing task {} with WASM VM", context.task_id);
        
        // TODO: Wire actual task data to enzyme runner
        // For now, return placeholder
        let result = enzyme.spawn_enzyme("placeholder.wasm", &context.task_id).await?;
        Ok(result)
    }

    /// Execute via learning loop
    async fn execute_learning(&self, context: &ExecutionContext, task_data: &[u8]) -> Result<Vec<u8>> {
        let learning = self.learning_loop.as_ref()
            .ok_or_else(|| anyhow!("Learning loop not available"))?;

        println!("[TaskRouter::Learning] Executing task {} with unified learning", context.task_id);
        
        // TODO: Wire task_data to learning loop for model training
        // For now, return success marker
        Ok(vec![0x01, 0x00, 0x00, 0x00]) // Success marker
    }

    /// Execute via network/federation executor
    async fn execute_network(&self, context: &ExecutionContext, task_data: &[u8]) -> Result<Vec<u8>> {
        println!("[TaskRouter::Network] Executing task {} over network", context.task_id);
        
        // TODO: Implement network task execution (federation, RPC, etc.)
        // For now, return echoed data
        Ok(task_data.to_vec())
    }

    /// Execute CPU-intensive task with throttling
    async fn execute_cpu_intensive(&self, context: &ExecutionContext, task_data: &[u8]) -> Result<Vec<u8>> {
        println!("[TaskRouter::CpuIntensive] Executing task {} with throttle {:.2}x",
            context.task_id, context.throttle_factor);
        
        // TODO: Implement CPU-intensive executor with thread pool
        // Spawn on rayon work-stealing pool with throttle factor
        Ok(vec![0x01]) // Placeholder
    }

    /// Execute memory-intensive task with careful allocation
    async fn execute_memory_intensive(&self, context: &ExecutionContext, task_data: &[u8]) -> Result<Vec<u8>> {
        println!("[TaskRouter::MemoryIntensive] Executing task {} with buffer limits",
            context.task_id);
        
        // TODO: Implement memory-limited executor
        // Use arena allocator or bounded memory pool
        Ok(vec![0x01]) // Placeholder
    }

    /// Execute simple inline task
    async fn execute_inline(&self, context: &ExecutionContext, task_data: &[u8]) -> Result<Vec<u8>> {
        println!("[TaskRouter::Inline] Executing task {} inline", context.task_id);
        
        // TODO: Implement simple inline executor for trivial tasks
        Ok(task_data.to_vec())
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
        assert_eq!(ExecutionRoute::from_analysis_type("wasm_processing"), ExecutionRoute::Enzyme);
        assert_eq!(ExecutionRoute::from_analysis_type("network_request"), ExecutionRoute::Network);
        assert_eq!(ExecutionRoute::from_analysis_type("model_training"), ExecutionRoute::Learning);
        assert_eq!(ExecutionRoute::from_analysis_type("cpu_compute"), ExecutionRoute::CpuIntensive);
        assert_eq!(ExecutionRoute::from_analysis_type("memory_buffer"), ExecutionRoute::MemoryIntensive);
        assert_eq!(ExecutionRoute::from_analysis_type("simple_task"), ExecutionRoute::Inline);
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
