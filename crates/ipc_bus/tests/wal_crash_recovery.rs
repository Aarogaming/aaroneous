use ipc_bus::persistent_wal::PersistentWalStore;
use std::fs::{File, OpenOptions};
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_wal_crash_recovery_mid_write_truncation() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("wal");

    let mut store = PersistentWalStore::open(&path).unwrap();
    store.put("key1", b"value1".to_vec()).unwrap();
    store.put("key2", b"value2".to_vec()).unwrap();
    drop(store);

    // Simulate mid-write truncation
    let file = OpenOptions::new().write(true).open(&path).unwrap();
    let original_len = file.metadata().unwrap().len();
    file.set_len(original_len - 4).unwrap();

    let store = PersistentWalStore::open(&path).unwrap();
    assert_eq!(store.get("key1"), Some(b"value1".as_slice()));
    assert_eq!(store.get("key2"), None);
}

#[test]
fn test_wal_crash_recovery_corrupted_magic_header() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("wal");

    let mut file = File::create(&path).unwrap();
    file.write_all(b"BAD!").unwrap(); // Corrupted magic header

    let result = PersistentWalStore::open(&path);
    assert!(result.is_err());
}

#[test]
fn test_wal_crash_recovery_tombstone_replay_and_compaction() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("wal");

    let mut store = PersistentWalStore::open(&path).unwrap();
    store.put("keyA", b"valueA".to_vec()).unwrap();
    store.put("keyB", b"valueB".to_vec()).unwrap();
    store.put("keyC", b"valueC".to_vec()).unwrap();
    store.delete("keyB").unwrap();
    drop(store);

    let mut store = PersistentWalStore::open(&path).unwrap();
    assert_eq!(store.get("keyA"), Some(b"valueA".as_slice()));
    assert_eq!(store.get("keyB"), None);
    assert_eq!(store.get("keyC"), Some(b"valueC".as_slice()));

    store.compact().unwrap();
    drop(store);

    let store = PersistentWalStore::open(&path).unwrap();
    assert_eq!(store.get("keyA"), Some(b"valueA".as_slice()));
    assert_eq!(store.get("keyB"), None);
    assert_eq!(store.get("keyC"), Some(b"valueC".as_slice()));

    let metadata = path.metadata().unwrap();
    assert!(metadata.len() < 1024);
}

#[test]
fn append_after_recovery_survives_the_next_restart() -> anyhow::Result<()> {
    // Exercise both an incomplete payload and an incomplete length prefix.
    for partial_prefix in [false, true] {
        let dir = tempdir()?;
        let path = dir.path().join("recovery.wal");
        let mut store = PersistentWalStore::open(&path)?;
        store.put("kept", b"first".to_vec())?;
        drop(store);
        let valid_end = path.metadata()?.len();
        let mut store = PersistentWalStore::open(&path)?;
        store.put("lost", b"second".to_vec())?;
        drop(store);
        let file = OpenOptions::new().write(true).open(&path)?;
        let end = if partial_prefix {
            valid_end + 2
        } else {
            file.metadata()?.len() - 4
        };
        file.set_len(end)?;
        drop(file);
        let mut store = PersistentWalStore::open(&path)?;
        assert_eq!(store.get("kept"), Some(b"first".as_slice()));
        assert_eq!(store.get("lost"), None);
        assert_eq!(path.metadata()?.len(), valid_end);
        store.put("new", b"third".to_vec())?;
        drop(store);
        let recovered = PersistentWalStore::open(&path)?;
        assert_eq!(recovered.get("kept"), Some(b"first".as_slice()));
        assert_eq!(recovered.get("new"), Some(b"third".as_slice()));
        assert_eq!(recovered.get("lost"), None);
    }
    Ok(())
}

#[test]
fn malicious_record_length_is_trimmed_before_allocation() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("oversized.wal");
    drop(PersistentWalStore::open(&path)?);
    let mut file = OpenOptions::new().append(true).open(&path)?;
    file.write_all(&u32::MAX.to_le_bytes())?;
    drop(file);
    let mut store = PersistentWalStore::open(&path)?;
    assert_eq!(path.metadata()?.len(), 6);
    store.put("after", b"valid".to_vec())?;
    drop(store);
    assert_eq!(
        PersistentWalStore::open(&path)?.get("after"),
        Some(b"valid".as_slice())
    );
    Ok(())
}

#[test]
fn corrupt_complete_record_does_not_truncate_existing_data() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("corrupt.wal");
    drop(PersistentWalStore::open(&path)?);
    let mut file = OpenOptions::new().append(true).open(&path)?;
    file.write_all(&4u32.to_le_bytes())?;
    file.write_all(b"BAD!")?;
    drop(file);
    let before = std::fs::read(&path)?;
    assert!(PersistentWalStore::open(&path).is_err());
    assert_eq!(std::fs::read(&path)?, before);
    Ok(())
}
