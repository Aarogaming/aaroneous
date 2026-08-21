use anyhow::Result;
use nervous_system::SharedMemorySynapse;
use wasmtime::{Engine, Linker, Module, Store};

/// The WASM Enzyme Loader.
/// Loads and executes WASM modules that can interact with the Shared Memory Synapse.
pub struct WasmEnzymeLoader {
    engine: Engine,
    linker: Linker<StoreData>,
}

pub struct StoreData {
    pub synapse: SharedMemorySynapse,
}

impl WasmEnzymeLoader {
    pub fn new() -> Result<Self> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);

        // Define the host function for writing to the Synapse
        // Note: wasm32-unknown-unknown places extern "C" imports in the "env" module by default
        linker.func_wrap(
            "env",
            "synapse_write",
            |mut caller: wasmtime::Caller<'_, StoreData>, offset: u32, ptr: u32, len: u32| {
                let mem = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return -1,
                };

                let mut buffer = vec![0u8; len as usize];
                if mem.read(&caller, ptr as usize, &mut buffer).is_err() {
                    return -2;
                }

                let data = caller.data_mut();
                // Use blocking runtime to call async write_at
                let rt = tokio::runtime::Runtime::new().unwrap();
                if rt
                    .block_on(data.synapse.write_at(offset as usize, &buffer))
                    .is_err()
                {
                    return -3;
                }
                0
            },
        )?;

        Ok(Self { engine, linker })
    }

    pub fn load_and_run(&self, wasm_path: &str) -> Result<()> {
        let module = Module::from_file(&self.engine, wasm_path)
            .map_err(|e| anyhow::anyhow!("Failed to load WASM module from {}: {}", wasm_path, e))?;

        let synapse = SharedMemorySynapse::new_sync("SAB_STORE", 1024 * 1024)?;
        let mut store = Store::new(&self.engine, StoreData { synapse });

        let instance = self.linker.instantiate(&mut store, &module)?;
        let run_fn = instance.get_typed_func::<(), i32>(&mut store, "run")?;

        let result = run_fn.call(&mut store, ())?;
        println!(
            "[WasmLoader] Enzyme executed successfully. Result: {}",
            result
        );
        Ok(())
    }
}
