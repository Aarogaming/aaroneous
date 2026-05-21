use sled::Db;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use crate::hox_map_schema::{EnzymeGenetics, HoxPermissions};

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

    pub fn get_enzyme(&self, name: &str) -> Option<EnzymeGenetics> {
        self.get_capability(name).ok().flatten().map(|cap| EnzymeGenetics {
            category: cap.name.clone(),
            expression_level: 1.0,
            permissions: HoxPermissions {
                max_sovereignty_tier: 0,
                allow_network: false,
                whitelisted_domains: vec![],
                requires_hitl: false,
            },
            mcp_tools: vec![],
        })
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
