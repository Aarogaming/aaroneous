

use nervous_system::SharedMemorySynapse;

/// The Storage Component.
/// Handles database persistence and event logging.
pub struct StorageEngine {
    #[allow(dead_code)]
    synapse: SharedMemorySynapse,
    db: Option<rusqlite::Connection>,
}

impl StorageEngine {
    pub fn new() -> Self {
        Self {
            synapse: SharedMemorySynapse::new_sync("SAB_STORE", 1024 * 1024).unwrap(),
            db: None,
        }
    }

    pub fn attach_db(&mut self, path: &str) -> Result<(), rusqlite::Error> {
        let conn = rusqlite::Connection::open(path)?;
        self.db = Some(conn);
        Ok(())
    }
}
