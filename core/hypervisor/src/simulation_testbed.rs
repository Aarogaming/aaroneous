use crate::enzyme_runner::WasmEnzymeRunner;
use nervous_system::shared_memory::SynapseState;
use anyhow::{Result, anyhow};
use std::path::Path;

pub struct SimulationTestbed {
    runner: WasmEnzymeRunner,
}

impl SimulationTestbed {
    pub fn new() -> Result<Self> {
        Ok(Self {
            runner: WasmEnzymeRunner::new()?,
        })
    }

    /// Tests a mutant chromosome in a sandboxed virtual synapse.
    /// Returns Ok(()) if the chromosome survives the pressure test.
    pub async fn pressure_test(&self, wasm_path: &Path, iterations: u32) -> Result<()> {
        println!("[SimulationTestbed] Commencing pressure test for: {}", wasm_path.display());

        // Create a virtual synapse state for the test
        let mut virtual_synapse = SynapseState::default();
        
        for i in 0..iterations {
            // Simulate increasing memory pressure and clock ticks
            virtual_synapse.clock_tick = i as u64;
            virtual_synapse.memory_pressure = (i as f32 / iterations as f32) * 100.0;

            // Execute the enzyme
            match self.runner.run_enzyme(wasm_path, &mut virtual_synapse).await {
                Ok(_) => {
                    // Check for internal safety locks triggered by the enzyme
                    if virtual_synapse.safety_lock == 1 {
                        return Err(anyhow!("Simulation failed: Enzyme triggered safety lock at iteration {}", i));
                    }
                    if virtual_synapse.error_sentinel == 1 {
                        return Err(anyhow!("Simulation failed: Enzyme reported error sentinel at iteration {}", i));
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Simulation failed: Runtime error at iteration {}: {}", i, e));
                }
            }
        }

        println!("[SimulationTestbed] Pressure test PASSED for: {}", wasm_path.display());
        Ok(())
    }
}
