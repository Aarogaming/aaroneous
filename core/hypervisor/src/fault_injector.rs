use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;
use nervous_system::SharedMemorySynapse;
use nervous_system::shared_memory::SynapseState;
use std::thread;
use rand::Rng;

/// Fault Injector (formerly ChaosMonkey)
pub struct FaultInjector {
    synapse: Arc<RwLock<SharedMemorySynapse>>,
}

impl FaultInjector {
    pub fn new(synapse: Arc<RwLock<SharedMemorySynapse>>) -> Self {
        Self { synapse }
    }

    /// Starts the injector thread. Controlled via the `fault_injector` feature flag.
    pub fn start(self) {
        println!("[FaultInjector] Initialized. Prepared to disrupt homeostasis.");
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            loop {
                // Randomly disrupt every 30‑60 seconds
                let sleep_secs = rng.gen_range(30..60);
                thread::sleep(Duration::from_secs(sleep_secs));

                let mut syn = self.synapse.write();
                let state_ptr = syn.get_ptr() as *mut SynapseState;
                // SAFETY: We know the pointer is valid for the lifetime of the synapse.
                let state = unsafe { &mut *state_ptr };

                match rng.gen_range(0..3) {
                    0 => {
                        println!("[FaultInjector] Injecting high memory pressure...");
                        state.memory_pressure = 95;
                    }
                    1 => {
                        println!("[FaultInjector] Flagging critical error sentinel...");
                        state.error_sentinel = 1;
                    }
                    2 => {
                        println!("[FaultInjector] Triggering phantom intent vector...");
                        state.intent_vector_id = [0xFF; 16];
                    }
                    _ => {}
                }
            }
        });
    }
}
