use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;
use nervous_system::SharedMemorySynapse;
use nervous_system::shared_memory::SynapseState;

pub struct ChaosMonkey {
    synapse: Arc<RwLock<SharedMemorySynapse>>,
}

impl ChaosMonkey {
    pub fn new(synapse: Arc<RwLock<SharedMemorySynapse>>) -> Self {
        Self { synapse }
    }

    pub fn start(self) {
        println!("[ChaosMonkey] Initialized. Prepared to disrupt homeostasis.");
        
        thread::spawn(move || {
            let mut rng = rand::rng();
            loop {
                // Randomly disrupt every 30-60 seconds
                let sleep_secs = rng.random_range(30..60);
                thread::sleep(Duration::from_secs(sleep_secs));
                
                let mut syn = self.synapse.write();
                let state_ptr = syn.get_ptr() as *mut SynapseState;
                let state = unsafe { &mut *state_ptr };

                match rng.random_range(0..3) {
                    0 => {
                        println!("[ChaosMonkey] Injecting high memory pressure...");
                        state.memory_pressure = 95;
                    }
                    1 => {
                        println!("[ChaosMonkey] Flagging critical error sentinel...");
                        state.error_sentinel = 1;
                    }
                    2 => {
                        println!("[ChaosMonkey] Triggering phantom intent vector...");
                        state.intent_vector_id = [0xFF; 16];
                    }
                    _ => {}
                }
            }
        });
    }
}

use std::thread;
use rand::RngExt;
