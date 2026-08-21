use crate::event_log::types::FederationEvent;
use anyhow::Result;
use rocksdb::{DB, Options};
use std::path::Path;

pub struct EventLogStore {
    db: DB,
}

impl EventLogStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path)?;
        Ok(Self { db })
    }

    pub fn append(&self, event: &FederationEvent) -> Result<()> {
        let key = event.event_id.as_bytes();
        let value = serde_json::to_vec(event)?;
        self.db.put(key, value)?;
        Ok(())
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<FederationEvent>> {
        if let Some(res) = self.db.get(id.as_bytes())? {
            let event: FederationEvent = serde_json::from_slice(&res)?;
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }
}
