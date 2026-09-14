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

