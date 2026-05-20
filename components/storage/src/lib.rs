// pub mod persistence; // Temporarily disabled: depends on hypervisor modules (skill_system, genetics, etc.)

use nervous_system::SharedMemorySynapse;

/// The Storage Component.
/// Handles database persistence and event logging.
pub struct StorageEngine {
    synapse: SharedMemorySynapse,
    db: Option<rusqlite::Connection>,
}

impl StorageEngine {
    pub fn new() -> Self {
        Self {
            synapse: SharedMemorySynapse::new("SAB_STORE", 1024 * 1024).unwrap(),
            db: None,
        }
    }

    pub fn attach_db(&mut self, path: &str) -> Result<(), rusqlite::Error> {
        let conn = rusqlite::Connection::open(path)?;
        self.db = Some(conn);
        Ok(())
    }
}
