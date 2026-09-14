use std::sync::Arc;
use std::thread;
use std::time::Duration;
use parking_lot::RwLock;
use rand::RngExt;
use ipc_bus::SharedMemorySynapse;
use ipc_bus::SynapseState;

pub struct ChaosMonkey {
    synapse: Arc<RwLock<SharedMemorySynapse>>,
}

impl ChaosMonkey {
    pub fn new(synapse: Arc<RwLock<SharedMemorySynapse>>) -> Self {
        Self { synapse }
    }

    /// Deterministically disrupt the synapse state once with the specified action:
    /// - 0: Inject high memory pressure (95)
    /// - 1: Flag critical safety lock sentinel (1)
    /// - 2: Trigger phantom intent vector ([0xFF; 16])
    pub fn disrupt_once(&self, action: u8) {
        let syn = self.synapse.write();
        let state_ptr = syn.get_ptr_sync() as *mut SynapseState;
        // SAFETY: pointer derived from verified active SharedMemorySynapse mapping
        let state = unsafe { &mut *state_ptr };

        match action {
            0 => {
                println!("[ChaosMonkey] Injecting high memory pressure...");
                state.memory_pressure = 95;
            }
            1 => {
                println!("[ChaosMonkey] Flagging critical safety lock sentinel...");
                state.safety_lock = 1;
            }
            2 => {
                println!("[ChaosMonkey] Triggering phantom intent vector...");
                state.intent_vector_id = [0xFF; 16];
            }
            _ => {}
        }
    }

    pub fn start(self) {
        println!("[ChaosMonkey] Initialized. Prepared to disrupt homeostasis.");
        
        thread::spawn(move || {
            let mut rng = rand::rng();
            loop {
                // Randomly disrupt every 30-60 seconds
                let sleep_secs = rng.random_range(30..60);
                thread::sleep(Duration::from_secs(sleep_secs));
                
                let action = rng.random_range(0..3);
                self.disrupt_once(action);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_chaos_monkey_creation() {
        let dir = tempdir().expect("create tempdir");
        let path = dir.path().join("test_synapse.bin");
        let synapse = SharedMemorySynapse::new_at(&path, 64 * 1024).expect("create synapse");
        let synapse = Arc::new(RwLock::new(synapse));
        let monkey = ChaosMonkey::new(Arc::clone(&synapse));
        monkey.disrupt_once(0);
        monkey.disrupt_once(1);
        monkey.disrupt_once(2);
    }
}
