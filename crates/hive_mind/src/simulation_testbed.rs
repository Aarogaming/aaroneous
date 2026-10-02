use anyhow::{Result, anyhow};
use hypervisor::enzyme_runner::WasmEnzymeRunner;
use ipc_bus::SynapseState;
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
        println!(
            "[SimulationTestbed] Commencing pressure test for: {}",
            wasm_path.display()
        );

        // Create a virtual synapse state for the test
        let mut virtual_synapse = SynapseState::default();

        for i in 0..iterations {
            // Simulate increasing memory pressure and clock ticks
            virtual_synapse.clock_tick = i as u64;
            virtual_synapse.memory_pressure = ((i * 100) / iterations.max(1)) as u8;

            // Execute the enzyme
            match self
                .runner
                .run_enzyme(wasm_path, &mut virtual_synapse)
                .await
            {
                Ok(_) => {
                    // Check for internal safety locks triggered by the enzyme
                    if virtual_synapse.safety_lock == 1 {
                        return Err(anyhow!(
                            "Simulation failed: Enzyme triggered safety lock at iteration {}",
                            i
                        ));
                    }
                }
                Err(e) => {
                    return Err(anyhow!(
                        "Simulation failed: Runtime error at iteration {}: {}",
                        i,
                        e
                    ));
                }
            }
        }

        println!(
            "[SimulationTestbed] Pressure test PASSED for: {}",
            wasm_path.display()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulation_testbed_creation() {
        let testbed = SimulationTestbed::new();
        assert!(testbed.is_ok());
    }

    #[tokio::test]
    async fn test_simulation_testbed_pressure_test() {
        let testbed = SimulationTestbed::new().unwrap();
        let path = Path::new("dummy_enzyme.wasm");
        let res = testbed.pressure_test(path, 3).await;
        assert!(res.is_ok());
    }
}
