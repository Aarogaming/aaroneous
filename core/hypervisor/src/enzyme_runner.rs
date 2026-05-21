use anyhow::Result;
use wasmtime::{Config, Engine, Module, Store};
use wasmtime::component::{Component, Linker as ComponentLinker, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

pub struct EnzymeRunner {
    engine: Engine,
    linker: ComponentLinker<EnzymeState>,
}

struct EnzymeState {
    wasi: WasiCtx,
    table: ResourceTable,
    synapse_id: String,
}

impl WasiView for EnzymeState {
    fn table(&mut self) -> &mut ResourceTable { &mut self.table }
    fn ctx(&mut self) -> &mut WasiCtx { &mut self.wasi }
}

impl EnzymeRunner {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        
        let engine = Engine::new(&config)?;
        let mut linker = ComponentLinker::new(&engine);
        
        // Add host imports here once wit-bindgen generates the traits
        
        Ok(Self { engine, linker })
    }

    pub async fn spawn_enzyme(&self, wasm_path: &str, task_id: &str) -> Result<Vec<u8>> {
        let component = Component::from_file(&self.engine, wasm_path)?;
        
        let mut builder = WasiCtxBuilder::new();
        builder.inherit_stdout().inherit_stderr();
        
        let state = EnzymeState {
            wasi: builder.build(),
            table: ResourceTable::new(),
            synapse_id: task_id.to_string(),
        };
        
        let mut store = Store::new(&self.engine, state);
        let instance = self.linker.instantiate_async(&mut store, &component).await?;
        
        // Assume the component has an export named "process-task"
        // In a real WIT-based setup, we'd use the generated Bindings
        let func = instance.get_func(&mut store, "process-task")
            .ok_or_else(|| anyhow!("Export 'process-task' not found"))?;
        
        // This is a simplified call. Real component calls involve Val/Param mapping.
        println!("[EnzymeRunner] Executing task {} in WASM...", task_id);
        
        Ok(vec![]) // Simulated result
    }

    pub async fn run_enzyme(
        &self,
        wasm_path: &std::path::Path,
        _synapse: &mut nervous_system::shared_memory::SynapseState,
    ) -> Result<()> {
        let task_id = wasm_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("enzyme");
        let _ = self.spawn_enzyme(&wasm_path.to_string_lossy(), task_id).await?;
        Ok(())
    }
}

/// Alias for backwards compatibility.
pub type WasmEnzymeRunner = EnzymeRunner;
