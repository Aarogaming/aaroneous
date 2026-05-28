use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use crate::hox_map_schema::{EnzymeGenetics, HoxPermissions};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HoxCapability {
    pub name: String,
    pub enzyme_hash: String,
    pub permissions: HoxPermissions,
}

pub struct HoxRegistry {
    db: Mutex<Connection>,
}

impl HoxRegistry {
    pub fn new(path: &str) -> Result<Self> {
        let db = Connection::open(path)?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS hox_capabilities (
                name TEXT PRIMARY KEY,
                enzyme_hash TEXT NOT NULL,
                permissions_json TEXT NOT NULL
            );",
        )?;
        Ok(Self { db: Mutex::new(db) })
    }

    pub fn register_capability(&self, cap: &HoxCapability) -> Result<()> {
        let permissions_json = serde_json::to_string(&cap.permissions)?;
        self.db.lock().execute(
            "INSERT INTO hox_capabilities (name, enzyme_hash, permissions_json)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(name) DO UPDATE SET
                 enzyme_hash = excluded.enzyme_hash,
                 permissions_json = excluded.permissions_json",
            params![cap.name, cap.enzyme_hash, permissions_json],
        )?;
        Ok(())
    }

    pub fn get_capability(&self, name: &str) -> Result<Option<HoxCapability>> {
        let db = self.db.lock();
        let mut stmt = db.prepare(
            "SELECT name, enzyme_hash, permissions_json FROM hox_capabilities WHERE name = ?1",
        )?;
        let row = stmt.query_row(params![name], |row| {
            let permissions_json: String = row.get(2)?;
            Ok(HoxCapability {
                name: row.get(0)?,
                enzyme_hash: row.get(1)?,
                permissions: serde_json::from_str(&permissions_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
            })
        });

        match row {
            Ok(cap) => Ok(Some(cap)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_enzyme(&self, name: &str) -> Option<EnzymeGenetics> {
        self.get_capability(name).ok().flatten().map(|cap| EnzymeGenetics {
            category: cap.name.clone(),
            expression_level: 1.0,
            permissions: cap.permissions.clone(),
            mcp_tools: vec![],
        })
    }

    pub fn list_capabilities(&self) -> Result<Vec<HoxCapability>> {
        let db = self.db.lock();
        let mut stmt = db.prepare(
            "SELECT name, enzyme_hash, permissions_json FROM hox_capabilities ORDER BY name",
        )?;
        let caps = stmt
            .query_map([], |row| {
                let permissions_json: String = row.get(2)?;
                Ok(HoxCapability {
                    name: row.get(0)?,
                    enzyme_hash: row.get(1)?,
                    permissions: serde_json::from_str(&permissions_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(caps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn capability(name: &str, enzyme_hash: &str, permissions: HoxPermissions) -> HoxCapability {
        HoxCapability {
            name: name.to_string(),
            enzyme_hash: enzyme_hash.to_string(),
            permissions,
        }
    }

    #[test]
    fn round_trips_capabilities_through_sqlite() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("hox.db");
        let registry = HoxRegistry::new(db_path.to_str().unwrap()).unwrap();

        let cap = capability(
            "odin",
            "hash-1",
            HoxPermissions {
                max_sovereignty_tier: 2,
                allow_network: true,
                whitelisted_domains: vec!["api.openai.com".to_string(), "api.anthropic.com".to_string()],
                requires_hitl: true,
            },
        );
        registry.register_capability(&cap).unwrap();

        let loaded = registry.get_capability("odin").unwrap().unwrap();
        assert_eq!(loaded.name, cap.name);
        assert_eq!(loaded.enzyme_hash, cap.enzyme_hash);
        assert_eq!(loaded.permissions.max_sovereignty_tier, cap.permissions.max_sovereignty_tier);
        assert_eq!(loaded.permissions.allow_network, cap.permissions.allow_network);
        assert_eq!(loaded.permissions.whitelisted_domains, cap.permissions.whitelisted_domains);
        assert_eq!(loaded.permissions.requires_hitl, cap.permissions.requires_hitl);

        let listed = registry.list_capabilities().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "odin");
    }

    #[test]
    fn overwrites_capabilities_and_orders_results() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("hox.db");
        let registry = HoxRegistry::new(db_path.to_str().unwrap()).unwrap();

        registry.register_capability(&capability(
            "merlin",
            "hash-a",
            HoxPermissions {
                max_sovereignty_tier: 1,
                allow_network: false,
                whitelisted_domains: vec!["signals.local".to_string()],
                requires_hitl: false,
            },
        )).unwrap();
        registry.register_capability(&capability(
            "odin",
            "hash-b",
            HoxPermissions {
                max_sovereignty_tier: 2,
                allow_network: true,
                whitelisted_domains: vec!["api.openai.com".to_string()],
                requires_hitl: true,
            },
        )).unwrap();
        registry.register_capability(&capability(
            "merlin",
            "hash-c",
            HoxPermissions {
                max_sovereignty_tier: 3,
                allow_network: true,
                whitelisted_domains: vec!["signals.local".to_string(), "api.anthropic.com".to_string()],
                requires_hitl: true,
            },
        )).unwrap();

        let merlin = registry.get_capability("merlin").unwrap().unwrap();
        assert_eq!(merlin.enzyme_hash, "hash-c");
        assert_eq!(merlin.permissions.max_sovereignty_tier, 3);
        assert!(merlin.permissions.allow_network);
        assert_eq!(merlin.permissions.whitelisted_domains, vec!["signals.local".to_string(), "api.anthropic.com".to_string()]);
        assert!(merlin.permissions.requires_hitl);

        let listed = registry.list_capabilities().unwrap();
        assert_eq!(listed.iter().map(|cap| cap.name.as_str()).collect::<Vec<_>>(), vec!["merlin", "odin"]);
    }

    #[test]
    fn get_enzyme_preserves_permissions() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("hox.db");
        let registry = HoxRegistry::new(db_path.to_str().unwrap()).unwrap();

        registry.register_capability(&capability(
            "odin",
            "hash-1",
            HoxPermissions {
                max_sovereignty_tier: 2,
                allow_network: true,
                whitelisted_domains: vec!["api.openai.com".to_string()],
                requires_hitl: true,
            },
        )).unwrap();

        let enzyme = registry.get_enzyme("odin").unwrap();
        assert_eq!(enzyme.category, "odin");
        assert_eq!(enzyme.permissions.max_sovereignty_tier, 2);
        assert!(enzyme.permissions.allow_network);
        assert_eq!(enzyme.permissions.whitelisted_domains, vec!["api.openai.com".to_string()]);
        assert!(enzyme.permissions.requires_hitl);
    }
}
