use anyhow::{Result, anyhow};
use wasmtime::{Config, Engine, Store};
use wasmtime::component::{Component, Linker as ComponentLinker, ResourceTable, Val};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, DirPerms, FilePerms};
use std::path::Path;

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
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl EnzymeState {
    pub fn reset(&mut self) {
        tracing::debug!("Resetting EnzymeState for synapse: {}", self.synapse_id);
    }
}

impl EnzymeRunner {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        
        let engine = Engine::new(&config)?;
        let mut linker = ComponentLinker::new(&engine);
        
        // Add WASI imports to the linker so the component can use them
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        
        Ok(Self { engine, linker })
    }

    pub async fn spawn_enzyme(&self, wasm_path: &str, task_id: &str) -> Result<Vec<u8>> {
        let component = Component::from_file(&self.engine, wasm_path)?;
        
        // Ensure the sandbox workspace exists
        let sandbox_path = Path::new("sandbox_workspace");
        if !sandbox_path.exists() {
            std::fs::create_dir_all(sandbox_path)?;
        }
        
        let mut builder = WasiCtxBuilder::new();
        // Strict Sandboxing: 
        // - No ambient network access
        // - No ambient file system access
        // - Only stdout/stderr for logging
        // - Only access to the mounted /workspace directory
        builder.inherit_stdout().inherit_stderr();
        
        builder.preopened_dir(sandbox_path, "/workspace", DirPerms::all(), FilePerms::all())?;
        
        let state = EnzymeState {
            wasi: builder.build(),
            table: ResourceTable::new(),
            synapse_id: task_id.to_string(),
        };
        
        let mut store = Store::new(&self.engine, state);
        let instance = self.linker.instantiate_async(&mut store, &component).await?;
        
        // Dynamic execution of the "process-task" export if it exists
        // In a real typed WIT environment, this would use generated bindings
        println!("[EnzymeRunner] Executing task {} in WASM Sandbox...", task_id);
        
        if let Some(func) = instance.get_func(&mut store, "process-task") {
            let mut results = [Val::S32(0)]; // Placeholder for standard return
            if let Err(e) = func.call_async(&mut store, &[], &mut results).await {
                println!("[EnzymeRunner] WASM Execution Error: {}", e);
                return Err(anyhow!("WASM Execution Error: {}", e));
            }
            println!("[EnzymeRunner] WASM Execution Completed Successfully.");
        } else {
            println!("[EnzymeRunner] Notice: 'process-task' export not found in WASM module. Initialization only.");
        }
        
        Ok(vec![]) // Simulated byte result
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
