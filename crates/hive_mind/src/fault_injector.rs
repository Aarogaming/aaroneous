use ipc_bus::SharedMemorySynapse;
use ipc_bus::shared_memory::IpcBusState;
use parking_lot::RwLock;
use rand::Rng;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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
        println!("[FaultInjector] Initialized.");
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            loop {
                // Randomly disrupt every 30-60 seconds
                let sleep_secs = rng.gen_range(30..60);
                thread::sleep(Duration::from_secs(sleep_secs));

                let syn = self.synapse.read();
                // SAFETY: pointer is valid for the lifetime of the synapse lock guard.
                let state_ptr = syn.get_ptr() as *mut IpcBusState;
                let state = unsafe { &mut *state_ptr };

                match rng.gen_range(0..3) {
                    0 => {
                        println!("[FaultInjector] Injecting high memory pressure...");
                        state.memory_pressure = 95;
                    }
                    1 => {
                        println!("[FaultInjector] Flagging safety lock...");
                        state.safety_lock = 1;
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
