// Multi-Runtime Tokio Governance
// Isolates I/O runtime (NATS, Raft, preparedness notices) from compute runtime
// (WASM execution, heavy compute, Python bindings) to prevent starvation.

use tokio::runtime::{Builder, Runtime};
use std::future::Future;

/// Task priority levels for routing decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskPriority {
    /// High-frequency I/O: NATS messaging, Raft heartbeats, preparedness notices
    HighFrequencyIO,
    /// Heavy compute: genome processing, tensor operations
    HeavyCompute,
    /// Python extension calls: GGUF harvesting, helix compilation
    PythonExtension,
    /// WASM guest execution: enzyme spawning, component calls
    WASMExecution,
}

/// Multi-runtime governance grid
pub struct RuntimeGovernor {
    /// Lean, responsive runtime for I/O and control plane
    io_runtime: Runtime,
    /// Larger runtime for compute-heavy workloads
    compute_runtime: Runtime,
}

impl RuntimeGovernor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // I/O Runtime: 2 threads, optimized for low-latency networking
        let io_runtime = Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("aaroneous-io")
            .thread_stack_size(2 * 1024 * 1024) // 2MB stack
            .enable_all()
            .build()?;

        // Compute Runtime: CPU-count threads, optimized for throughput
        let cpu_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let compute_runtime = Builder::new_multi_thread()
            .worker_threads(cpu_count.max(4))
            .thread_name("aaroneous-compute")
            .thread_stack_size(8 * 1024 * 1024) // 8MB stack for deep recursion
            .enable_all()
            .build()?;

        Ok(Self {
            io_runtime,
            compute_runtime,
        })
    }

    /// Execute a task on the appropriate runtime based on priority
    pub async fn execute<F, T>(&self, priority: TaskPriority, task: F) -> T
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        match priority {
            TaskPriority::HighFrequencyIO => {
                // Cooperatively yield to ensure NATS/Raft heartbeats never drop
                tokio::task::yield_now().await;
                self.io_runtime.spawn(async move { task() }).await.unwrap()
            }
            TaskPriority::HeavyCompute | TaskPriority::PythonExtension | TaskPriority::WASMExecution => {
                // Offload entirely to blocking pool to isolate Tokio from CPU starvation
                self.compute_runtime
                    .spawn_blocking(move || task())
                    .await
                    .unwrap()
            }
        }
    }

    /// Spawn an async task on the I/O runtime
    pub fn spawn_io<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.io_runtime.spawn(future)
    }

    /// Spawn a blocking task on the compute runtime
    pub fn spawn_compute<F, T>(&self, task: F) -> tokio::task::JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.compute_runtime.spawn_blocking(task)
    }

    /// Get a handle to the I/O runtime for direct use
    pub fn io_handle(&self) -> &Runtime {
        &self.io_runtime
    }

    /// Get a handle to the compute runtime for direct use
    pub fn compute_handle(&self) -> &Runtime {
        &self.compute_runtime
    }

    /// Shutdown both runtimes gracefully
    pub fn shutdown(self) {
        self.io_runtime.shutdown_background();
        self.compute_runtime.shutdown_background();
    }
}

/// Budget-aware task executor for preventing tight loops from monopolizing threads
pub struct BudgetExecutor;

impl BudgetExecutor {
    /// Execute with periodic yielding to prevent thread starvation
    pub async fn execute_with_budget<F, T>(task: F) -> T
    where
        F: FnOnce() -> T,
    {
        // Yield before execution to give other tasks a chance
        tokio::task::yield_now().await;
        task()
    }

    /// Execute a packet-processing loop with periodic yielding
    pub async fn process_loop<F, Fut, T>(mut process: F)
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Option<T>>,
    {
        let mut iterations = 0;
        const YIELD_INTERVAL: u32 = 64; // Yield every 64 iterations

        loop {
            match process().await {
                Some(_) => {
                    iterations += 1;
                    if iterations % YIELD_INTERVAL == 0 {
                        tokio::task::yield_now().await;
                    }
                }
                None => break,
            }
        }
    }
}

/// Task routing helper for agent execution
pub async fn execute_agent_task<F, T>(task_type: TaskPriority, compute_payload: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match task_type {
        TaskPriority::HighFrequencyIO => {
            tokio::task::yield_now().await;
            tokio::task::spawn(async move { compute_payload() }).await.unwrap()
        }
        TaskPriority::HeavyCompute | TaskPriority::PythonExtension | TaskPriority::WASMExecution => {
            tokio::task::spawn_blocking(move || compute_payload()).await.unwrap()
        }
    }
}
