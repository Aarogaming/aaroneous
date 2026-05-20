use sled::Db;
use serde::{Serialize, Deserialize};
use anyhow::Result;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HoxCapability {
    pub name: String,
    pub enzyme_hash: String,
    pub permissions: Vec<String>,
}

pub struct HoxRegistry {
    db: Db,
}

impl HoxRegistry {
    pub fn new(path: &str) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn register_capability(&self, cap: &HoxCapability) -> Result<()> {
        let value = serde_json::to_vec(cap)?;
        self.db.insert(&cap.name, value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn get_capability(&self, name: &str) -> Result<Option<HoxCapability>> {
        if let Some(ivec) = self.db.get(name)? {
            let cap: HoxCapability = serde_json::from_slice(&ivec)?;
            Ok(Some(cap))
        } else {
            Ok(None)
        }
    }

    pub fn list_capabilities(&self) -> Result<Vec<HoxCapability>> {
        let mut caps = Vec::new();
        for item in self.db.iter() {
            let (_key, value) = item?;
            let cap: HoxCapability = serde_json::from_slice(&value)?;
            caps.push(cap);
        }
        Ok(caps)
    }
}
