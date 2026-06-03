/// Hox Registry Persistence - save and load full registry state to disk
///
/// Extends HoxRegistry with comprehensive serialization/deserialization
/// for checkpointing and recovery across restarts.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, anyhow};
use crate::hox_registry::{HoxRegistry, HoxCapability};
use crate::hox_map_schema::HoxPermissions;

/// Persistent registry snapshot for disk storage
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistrySnapshot {
    pub version: u32,
    pub timestamp: u64,
    pub capabilities: Vec<HoxCapability>,
    pub metadata: RegistryMetadata,
}

/// Metadata about the registry state
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistryMetadata {
    pub total_entries: usize,
    pub last_modified: u64,
    pub checksum: String,
}

impl RegistrySnapshot {
    /// Create new snapshot
    pub fn new(capabilities: Vec<HoxCapability>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        let checksum = Self::compute_checksum(&capabilities);
        
        Self {
            version: 1,
            timestamp: now,
            capabilities: capabilities.clone(),
            metadata: RegistryMetadata {
                total_entries: capabilities.len(),
                last_modified: now,
                checksum,
            },
        }
    }

    /// Compute checksum for integrity verification
    fn compute_checksum(capabilities: &[HoxCapability]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        for cap in capabilities {
            cap.name.hash(&mut hasher);
            cap.enzyme_hash.hash(&mut hasher);
        }
        
        format!("{:x}", hasher.finish())
    }

    /// Verify checksum integrity
    pub fn verify_integrity(&self) -> bool {
        let computed = Self::compute_checksum(&self.capabilities);
        computed == self.metadata.checksum
    }
}

/// Hox Registry Persistence Handler
pub struct HoxPersistenceManager {
    registry: HoxRegistry,
    snapshot_dir: PathBuf,
}

impl HoxPersistenceManager {
    /// Create new persistence manager
    pub fn new(db_path: &str, snapshot_dir: &str) -> Result<Self> {
        let registry = HoxRegistry::new(db_path)?;
        let snapshot_dir = PathBuf::from(snapshot_dir);
        
        // Create snapshot directory if it doesn't exist
        if !snapshot_dir.exists() {
            fs::create_dir_all(&snapshot_dir)?;
        }
        
        Ok(Self {
            registry,
            snapshot_dir,
        })
    }

    /// Save current registry state to disk
    pub fn save_snapshot(&self, name: Option<&str>) -> Result<PathBuf> {
        let capabilities = self.registry.list_capabilities()?;
        let snapshot = RegistrySnapshot::new(capabilities);
        
        // Verify integrity before saving
        if !snapshot.verify_integrity() {
            return Err(anyhow!("Registry integrity check failed before save"));
        }
        
        let filename = name.unwrap_or_else(|| {
            &format!("snapshot_{}.json", snapshot.timestamp)
        });
        
        let path = self.snapshot_dir.join(filename);
        
        let json = serde_json::to_string_pretty(&snapshot)?;
        fs::write(&path, json)?;
        
        println!("[HoxPersistence] Snapshot saved: {} ({} entries)",
            path.display(), snapshot.metadata.total_entries);
        
        Ok(path)
    }

    /// Load snapshot from disk
    pub fn load_snapshot(&self, path: &Path) -> Result<RegistrySnapshot> {
        let json = fs::read_to_string(path)?;
        let snapshot: RegistrySnapshot = serde_json::from_str(&json)?;
        
        // Verify integrity
        if !snapshot.verify_integrity() {
            return Err(anyhow!("Snapshot integrity check failed: corrupted data"));
        }
        
        println!("[HoxPersistence] Snapshot loaded: {} ({} entries)",
            path.display(), snapshot.metadata.total_entries);
        
        Ok(snapshot)
    }

    /// Restore registry from snapshot
    pub fn restore_from_snapshot(&self, snapshot: &RegistrySnapshot) -> Result<()> {
        for capability in &snapshot.capabilities {
            self.registry.register_capability(capability)?;
        }
        
        println!("[HoxPersistence] Restored {} capabilities from snapshot",
            snapshot.metadata.total_entries);
        
        Ok(())
    }

    /// List all available snapshots
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>> {
        let mut snapshots = Vec::new();
        
        for entry in fs::read_dir(&self.snapshot_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(snapshot) = self.load_snapshot(&path) {
                    snapshots.push(SnapshotInfo {
                        path,
                        timestamp: snapshot.timestamp,
                        entries: snapshot.metadata.total_entries,
                    });
                }
            }
        }
        
        // Sort by timestamp descending
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        Ok(snapshots)
    }

    /// Auto-save snapshot at regular intervals
    pub fn auto_save(&self) -> Result<PathBuf> {
        self.save_snapshot(None)
    }

    /// Create backup before modification
    pub fn create_backup(&self) -> Result<PathBuf> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        let name = format!("backup_{}.json", timestamp);
        self.save_snapshot(Some(&name))
    }

    /// Get registry statistics
    pub fn get_stats(&self) -> Result<RegistryStats> {
        let capabilities = self.registry.list_capabilities()?;
        let snapshots = self.list_snapshots()?;
        
        Ok(RegistryStats {
            total_capabilities: capabilities.len(),
            total_snapshots: snapshots.len(),
            latest_snapshot: snapshots.first().map(|s| s.timestamp),
            snapshot_dir: self.snapshot_dir.clone(),
        })
    }
}

/// Information about a snapshot
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    pub path: PathBuf,
    pub timestamp: u64,
    pub entries: usize,
}

/// Registry statistics
#[derive(Debug, Clone, Serialize)]
pub struct RegistryStats {
    pub total_capabilities: usize,
    pub total_snapshots: usize,
    pub latest_snapshot: Option<u64>,
    pub snapshot_dir: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_snapshot_creation() {
        let cap = HoxCapability {
            name: "test_enzyme".to_string(),
            enzyme_hash: "hash123".to_string(),
            permissions: HoxPermissions {
                max_sovereignty_tier: 2,
                allow_network: true,
                whitelisted_domains: vec!["example.com".to_string()],
                requires_hitl: false,
            },
        };
        
        let snapshot = RegistrySnapshot::new(vec![cap]);
        assert_eq!(snapshot.metadata.total_entries, 1);
        assert!(snapshot.verify_integrity());
    }

    #[test]
    fn test_snapshot_serialization() {
        let cap = HoxCapability {
            name: "test_enzyme".to_string(),
            enzyme_hash: "hash123".to_string(),
            permissions: HoxPermissions {
                max_sovereignty_tier: 2,
                allow_network: true,
                whitelisted_domains: vec!["example.com".to_string()],
                requires_hitl: false,
            },
        };
        
        let snapshot = RegistrySnapshot::new(vec![cap]);
        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: RegistrySnapshot = serde_json::from_str(&json).unwrap();
        
        assert_eq!(restored.metadata.total_entries, snapshot.metadata.total_entries);
        assert_eq!(restored.metadata.checksum, snapshot.metadata.checksum);
    }

    #[test]
    fn test_persistence_manager() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("hox.db");
        let snap_dir = temp_dir.path().join("snapshots");
        
        let manager = HoxPersistenceManager::new(
            db_path.to_str().unwrap(),
            snap_dir.to_str().unwrap(),
        ).unwrap();
        
        // Create snapshot
        let snapshot_path = manager.auto_save().unwrap();
        assert!(snapshot_path.exists());
        
        // List snapshots
        let snapshots = manager.list_snapshots().unwrap();
        assert_eq!(snapshots.len(), 1);
    }
}
