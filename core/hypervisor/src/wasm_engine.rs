use wasmtime::*;
use anyhow::Result;
use std::path::Path;

pub struct WasmEnzymeHost {
    engine: Engine,
    linker: Linker<()>,
}

pub struct EnzymeInstance {
    pub instance: Instance,
    pub store: Store<()>,
}

impl WasmEnzymeHost {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_threads(true);
        config.consume_fuel(true); // For instruction-level governance
        
        let engine = Engine::new(&config)?;
        let linker = Linker::new(&engine);
        
        Ok(Self { engine, linker })
    }

    pub fn instantiate_enzyme(&self, path: &Path, fuel: u64) -> Result<EnzymeInstance> {
        let module = Module::from_file(&self.engine, path)?;
        let mut store = Store::new(&self.engine, ());
        store.set_fuel(fuel)?; 
        
        let instance = self.linker.instantiate(&mut store, &module)?;
        Ok(EnzymeInstance { instance, store })
    }

    /// Execute a standard metabolic enzyme calculation
    pub fn run_metabolic_update(&self, enzyme: &mut EnzymeInstance) -> Result<f32> {
        let func = enzyme.instance.get_typed_func::<(), f32>(&mut enzyme.store, "calculate_metabolism")?;
        let result = func.call(&mut enzyme.store, ())?;
        Ok(result)
    }
}
