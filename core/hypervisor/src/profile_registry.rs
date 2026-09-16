use crate::profile_schema::{EnzymeGenetics, NodePermissions};
use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeCapability {
    pub name: String,
    pub enzyme_hash: String,
    pub permissions: NodePermissions,
}

/// Modern capability descriptor alias
pub type CapabilityDescriptor = NodeCapability;
pub type HoxCapability = NodeCapability;

pub struct ProfileRegistry {
    db: Mutex<Connection>,
}

/// Modern capability schema registry alias
pub type CapabilitySchemaRegistry = ProfileRegistry;
pub type HoxRegistry = ProfileRegistry;

impl ProfileRegistry {
    pub fn new(path: &str) -> Result<Self> {
        let db = Connection::open(path)?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS node_capabilities (
                name TEXT PRIMARY KEY,
                enzyme_hash TEXT NOT NULL,
                permissions_json TEXT NOT NULL
            );",
        )?;
        Ok(Self { db: Mutex::new(db) })
    }

    pub fn register_capability(&self, cap: &NodeCapability) -> Result<()> {
        let permissions_json = serde_json::to_string(&cap.permissions)?;
        self.db.lock().execute(
            "INSERT INTO node_capabilities (name, enzyme_hash, permissions_json)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(name) DO UPDATE SET
                 enzyme_hash = excluded.enzyme_hash,
                 permissions_json = excluded.permissions_json",
            params![cap.name, cap.enzyme_hash, permissions_json],
        )?;
        Ok(())
    }

    pub fn get_capability(&self, name: &str) -> Result<Option<NodeCapability>> {
        let db = self.db.lock();
        let mut stmt = db.prepare(
            "SELECT name, enzyme_hash, permissions_json FROM node_capabilities WHERE name = ?1",
        )?;
        let row = stmt.query_row(params![name], |row| {
            let permissions_json: String = row.get(2)?;
            Ok(NodeCapability {
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

    pub fn get_module(&self, name: &str) -> Option<EnzymeGenetics> {
        self.get_capability(name)
            .ok()
            .flatten()
            .map(|cap| EnzymeGenetics {
                category: cap.name.clone(),
                expression_level: 1.0,
                permissions: cap.permissions.clone(),
                mcp_tools: vec![],
            })
    }

    pub fn list_capabilities(&self) -> Result<Vec<NodeCapability>> {
        let db = self.db.lock();
        let mut stmt = db.prepare(
            "SELECT name, enzyme_hash, permissions_json FROM node_capabilities ORDER BY name",
        )?;
        let caps = stmt
            .query_map([], |row| {
                let permissions_json: String = row.get(2)?;
                Ok(NodeCapability {
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

    fn capability(name: &str, enzyme_hash: &str, permissions: NodePermissions) -> NodeCapability {
        NodeCapability {
            name: name.to_string(),
            enzyme_hash: enzyme_hash.to_string(),
            permissions,
        }
    }

    #[test]
    fn round_trips_capabilities_through_sqlite() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("hox.db");
        let registry = ProfileRegistry::new(db_path.to_str().unwrap()).unwrap();

        let cap = capability(
            "orchestrator",
            "hash-1",
            NodePermissions {
                max_sovereignty_tier: 2,
                allow_network: true,
                whitelisted_domains: vec![
                    "api.openai.com".to_string(),
                    "api.anthropic.com".to_string(),
                ],
                requires_hitl: true,
            },
        );
        registry.register_capability(&cap).unwrap();

        let loaded = registry.get_capability("orchestrator").unwrap().unwrap();
        assert_eq!(loaded.name, cap.name);
        assert_eq!(loaded.enzyme_hash, cap.enzyme_hash);
        assert_eq!(
            loaded.permissions.max_sovereignty_tier,
            cap.permissions.max_sovereignty_tier
        );
        assert_eq!(
            loaded.permissions.allow_network,
            cap.permissions.allow_network
        );
        assert_eq!(
            loaded.permissions.whitelisted_domains,
            cap.permissions.whitelisted_domains
        );
        assert_eq!(
            loaded.permissions.requires_hitl,
            cap.permissions.requires_hitl
        );

        let listed = registry.list_capabilities().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "orchestrator");
    }

    #[test]
    fn overwrites_capabilities_and_orders_results() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("hox.db");
        let registry = ProfileRegistry::new(db_path.to_str().unwrap()).unwrap();

        registry
            .register_capability(&capability(
                "synthesizer",
                "hash-a",
                NodePermissions {
                    max_sovereignty_tier: 1,
                    allow_network: false,
                    whitelisted_domains: vec!["signals.local".to_string()],
                    requires_hitl: false,
                },
            ))
            .unwrap();
        registry
            .register_capability(&capability(
                "orchestrator",
                "hash-b",
                NodePermissions {
                    max_sovereignty_tier: 2,
                    allow_network: true,
                    whitelisted_domains: vec!["api.openai.com".to_string()],
                    requires_hitl: true,
                },
            ))
            .unwrap();
        registry
            .register_capability(&capability(
                "synthesizer",
                "hash-c",
                NodePermissions {
                    max_sovereignty_tier: 3,
                    allow_network: true,
                    whitelisted_domains: vec![
                        "signals.local".to_string(),
                        "api.anthropic.com".to_string(),
                    ],
                    requires_hitl: true,
                },
            ))
            .unwrap();

        let synthesizer = registry.get_capability("synthesizer").unwrap().unwrap();
        assert_eq!(synthesizer.enzyme_hash, "hash-c");
        assert_eq!(synthesizer.permissions.max_sovereignty_tier, 3);
        assert!(synthesizer.permissions.allow_network);
        assert_eq!(
            synthesizer.permissions.whitelisted_domains,
            vec!["signals.local".to_string(), "api.anthropic.com".to_string()]
        );
        assert!(synthesizer.permissions.requires_hitl);

        let listed = registry.list_capabilities().unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|cap| cap.name.as_str())
                .collect::<Vec<_>>(),
            vec!["orchestrator", "synthesizer"]
        );
    }

    #[test]
    fn get_module_preserves_permissions() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("hox.db");
        let registry = ProfileRegistry::new(db_path.to_str().unwrap()).unwrap();

        registry
            .register_capability(&capability(
                "orchestrator",
                "hash-1",
                NodePermissions {
                    max_sovereignty_tier: 2,
                    allow_network: true,
                    whitelisted_domains: vec!["api.openai.com".to_string()],
                    requires_hitl: true,
                },
            ))
            .unwrap();

        let enzyme = registry.get_module("orchestrator").unwrap();
        assert_eq!(enzyme.category, "orchestrator");
        assert_eq!(enzyme.permissions.max_sovereignty_tier, 2);
        assert!(enzyme.permissions.allow_network);
        assert_eq!(
            enzyme.permissions.whitelisted_domains,
            vec!["api.openai.com".to_string()]
        );
        assert!(enzyme.permissions.requires_hitl);
    }
}
