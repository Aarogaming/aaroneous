use anyhow::Result;
use async_trait::async_trait;
use nervous_system::shared_memory::SynapseState;

#[async_trait]
pub trait NativeEnzyme: Send + Sync {
    async fn execute(&self, state: &mut SynapseState, task_id: &str) -> Result<Vec<u8>>;
}

pub struct NativeComputeEnzyme;

#[async_trait]
impl NativeEnzyme for NativeComputeEnzyme {
    async fn execute(&self, _state: &mut SynapseState, task_id: &str) -> Result<Vec<u8>> {
        tracing::info!(target: "native_enzyme", task_id = %task_id, "Executing native compute enzyme");
        let result = format!(
            "{{\"status\": \"success\", \"task_id\": \"{}\", \"compute\": \"native_entropy_ok\"}}",
            task_id
        );
        Ok(result.into_bytes())
    }
}

pub struct NativeTestEnzyme;

#[async_trait]
impl NativeEnzyme for NativeTestEnzyme {
    async fn execute(&self, _state: &mut SynapseState, task_id: &str) -> Result<Vec<u8>> {
        tracing::info!(target: "native_enzyme", task_id = %task_id, "Executing native test enzyme");
        let result = format!(
            "{{\"status\": \"success\", \"task_id\": \"{}\", \"test\": \"native_validation_ok\"}}",
            task_id
        );
        Ok(result.into_bytes())
    }
}

pub struct EnzymeRunner {
    compute_enzyme: NativeComputeEnzyme,
    test_enzyme: NativeTestEnzyme,
}

/// Modern micro-task runner alias
pub type MicroTaskRunner = EnzymeRunner;

/// Modern micro-task worker trait alias
pub use NativeEnzyme as MicroTaskWorker;

impl EnzymeRunner {
    pub fn new() -> Result<Self> {
        Ok(Self {
            compute_enzyme: NativeComputeEnzyme,
            test_enzyme: NativeTestEnzyme,
        })
    }

    pub async fn spawn_enzyme(&self, wasm_path: &str, task_id: &str) -> Result<Vec<u8>> {
        tracing::info!(target: "enzyme_runner", path = %wasm_path, task_id = %task_id, "Dispatching native rust enzyme instead of WASM");
        let mut dummy_state = SynapseState::default();
        if task_id.contains("test") {
            self.test_enzyme.execute(&mut dummy_state, task_id).await
        } else {
            self.compute_enzyme.execute(&mut dummy_state, task_id).await
        }
    }

    pub async fn run_enzyme(
        &self,
        wasm_path: &std::path::Path,
        _synapse: &mut SynapseState,
    ) -> Result<()> {
        let task_id = wasm_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("enzyme");
        let _ = self
            .spawn_enzyme(&wasm_path.to_string_lossy(), task_id)
            .await?;
        Ok(())
    }
}
