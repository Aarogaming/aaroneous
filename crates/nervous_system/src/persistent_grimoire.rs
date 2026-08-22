//! crates/nervous_system/src/persistent_grimoire.rs
//! High-Performance Embedded ACID Key-Value & Intent Persistence Engine.
//! Provides durability across daemon reboots, intent history tracking, and specialist skill persistence.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const GRIMOIRE_MAGIC: &[u8; 4] = b"GRIM";
const GRIMOIRE_VERSION: u16 = 1;

/// A durable record stored in the Grimoire WAL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrimoireRecord {
    pub key: String,
    pub value: Vec<u8>,
    pub generation: u64,
    pub timestamp_ms: u64,
    pub is_tombstone: bool,
}

/// Persistent Grimoire Key-Value & Intent Store
pub struct PersistentGrimoireStore {
    db_path: PathBuf,
    wal_file: BufWriter<File>,
    index: BTreeMap<String, Vec<u8>>,
    current_generation: u64,
}

impl PersistentGrimoireStore {
    /// Opens or creates a durable Grimoire database at the given path
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self> {
        let path = db_path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut index = BTreeMap::new();
        let mut max_generation = 0u64;

        // Replay existing WAL if file exists and has content
        if path.exists() {
            let read_file = File::open(&path)?;
            if read_file.metadata()?.len() >= 6 {
                let mut reader = BufReader::new(read_file);
                let mut magic = [0u8; 4];
                reader.read_exact(&mut magic)?;
                if &magic != GRIMOIRE_MAGIC {
                    return Err(anyhow!("Invalid Grimoire database magic bytes"));
                }
                let mut version_bytes = [0u8; 2];
                reader.read_exact(&mut version_bytes)?;
                let version = u16::from_le_bytes(version_bytes);
                if version != GRIMOIRE_VERSION {
                    return Err(anyhow!("Unsupported Grimoire database version: {}", version));
                }

                // Read records iteratively until EOF
                while let Ok(len_bytes) = {
                    let mut b = [0u8; 4];
                    reader.read_exact(&mut b).map(|_| u32::from_le_bytes(b))
                } {
                    let mut record_bytes = vec![0u8; len_bytes as usize];
                    reader.read_exact(&mut record_bytes)?;
                    if let Ok(record) = serde_json::from_slice::<GrimoireRecord>(&record_bytes) {
                        max_generation = max_generation.max(record.generation);
                        if record.is_tombstone {
                            index.remove(&record.key);
                        } else {
                            index.insert(record.key, record.value);
                        }
                    }
                }
            }
        }

        // Open WAL in append mode
        let write_file = OpenOptions::new()
            .read(true)
            .create(true)
            .append(true)
            .open(&path)?;

        let mut wal_file = BufWriter::new(write_file);

        // If file was brand new, write magic header
        if wal_file.get_ref().metadata()?.len() == 0 {
            wal_file.write_all(GRIMOIRE_MAGIC)?;
            wal_file.write_all(&GRIMOIRE_VERSION.to_le_bytes())?;
            wal_file.flush()?;
        }

        Ok(Self {
            db_path: path,
            wal_file,
            index,
            current_generation: max_generation,
        })
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Persists a key-value pair and immediately updates in-memory index
    pub fn put(&mut self, key: impl Into<String>, value: impl Into<Vec<u8>>) -> Result<()> {
        let key_str = key.into();
        let val_bytes = value.into();
        self.current_generation += 1;

        let record = GrimoireRecord {
            key: key_str.clone(),
            value: val_bytes.clone(),
            generation: self.current_generation,
            timestamp_ms: Self::now_ms(),
            is_tombstone: false,
        };

        let serialized = serde_json::to_vec(&record)?;
        let len_bytes = (serialized.len() as u32).to_le_bytes();

        self.wal_file.write_all(&len_bytes)?;
        self.wal_file.write_all(&serialized)?;
        self.wal_file.flush()?;
        self.wal_file.get_ref().sync_data()?;

        self.index.insert(key_str, val_bytes);
        Ok(())
    }

    /// Retrieves a value by key from the in-memory cache
    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.index.get(key).map(|v| v.as_slice())
    }

    /// Deletes a key by appending a tombstone record
    pub fn delete(&mut self, key: &str) -> Result<()> {
        if !self.index.contains_key(key) {
            return Ok(());
        }

        self.current_generation += 1;
        let record = GrimoireRecord {
            key: key.to_string(),
            value: Vec::new(),
            generation: self.current_generation,
            timestamp_ms: Self::now_ms(),
            is_tombstone: true,
        };

        let serialized = serde_json::to_vec(&record)?;
        let len_bytes = (serialized.len() as u32).to_le_bytes();

        self.wal_file.write_all(&len_bytes)?;
        self.wal_file.write_all(&serialized)?;
        self.wal_file.flush()?;
        self.wal_file.get_ref().sync_data()?;

        self.index.remove(key);
        Ok(())
    }

    /// Lists all keys starting with a prefix
    pub fn list_keys_with_prefix(&self, prefix: &str) -> Vec<String> {
        self.index
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect()
    }

    /// Returns total active key count
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Returns whether the store contains no active keys.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Returns current generation tick
    pub fn generation(&self) -> u64 {
        self.current_generation
    }

    /// Compacts the WAL file by writing only the latest live active entries
    pub fn compact(&mut self) -> Result<()> {
        let temp_path = self.db_path.with_extension("compact.tmp");
        {
            let mut temp_file = BufWriter::new(
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&temp_path)?,
            );

            temp_file.write_all(GRIMOIRE_MAGIC)?;
            temp_file.write_all(&GRIMOIRE_VERSION.to_le_bytes())?;

            for (k, v) in &self.index {
                let record = GrimoireRecord {
                    key: k.clone(),
                    value: v.clone(),
                    generation: self.current_generation,
                    timestamp_ms: Self::now_ms(),
                    is_tombstone: false,
                };
                let serialized = serde_json::to_vec(&record)?;
                let len_bytes = (serialized.len() as u32).to_le_bytes();
                temp_file.write_all(&len_bytes)?;
                temp_file.write_all(&serialized)?;
            }
            temp_file.flush()?;
            temp_file.get_ref().sync_all()?;
        }

        // Replace original with compacted file
        std::fs::rename(&temp_path, &self.db_path)?;

        // Re-open WAL handle
        let write_file = OpenOptions::new()
            .read(true)
            .create(true)
            .append(true)
            .open(&self.db_path)?;

        self.wal_file = BufWriter::new(write_file);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistent_grimoire_reboot_durability() {
        let temp_dir = std::env::temp_dir().join(format!("grimoire_test_{}", PersistentGrimoireStore::now_ms()));
        let db_path = temp_dir.join("grimoire.db");

        // 1. Write records in session 1
        {
            let mut store = PersistentGrimoireStore::open(&db_path).unwrap();
            store.put("skill://merlin/fireball", b"rank_s").unwrap();
            store.put("intent://odin/001", b"consensus_reached").unwrap();
            store.put("memory://temp", b"to_be_deleted").unwrap();
            store.delete("memory://temp").unwrap();
            assert_eq!(store.len(), 2);
        }

        // 2. Re-open (simulate system restart)
        {
            let store = PersistentGrimoireStore::open(&db_path).unwrap();
            assert_eq!(store.len(), 2);
            assert_eq!(store.get("skill://merlin/fireball"), Some(b"rank_s".as_slice()));
            assert_eq!(store.get("intent://odin/001"), Some(b"consensus_reached".as_slice()));
            assert_eq!(store.get("memory://temp"), None);

            let merlin_skills = store.list_keys_with_prefix("skill://merlin");
            assert_eq!(merlin_skills.len(), 1);
        }

        // 3. Compact database
        {
            let mut store = PersistentGrimoireStore::open(&db_path).unwrap();
            store.compact().unwrap();
            assert_eq!(store.len(), 2);
            assert_eq!(store.get("skill://merlin/fireball"), Some(b"rank_s".as_slice()));
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
