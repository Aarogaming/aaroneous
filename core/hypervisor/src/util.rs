// src/util.rs
//! Utility helpers for safe lock acquisition and error mapping in the hypervisor ACC.

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::error::HypervisorError;

/// Acquire a write lock on an `RwLock`, mapping any poisoning error to `HypervisorError`.
pub fn lock_write<T>(lock: &RwLock<T>) -> Result<RwLockWriteGuard<T>, HypervisorError> {
    lock.write().map_err(|e| HypervisorError::RuntimeError(e.to_string()))
}

/// Acquire a read lock on an `RwLock`, mapping any poisoning error to `HypervisorError`.
pub fn lock_read<T>(lock: &RwLock<T>) -> Result<RwLockReadGuard<T>, HypervisorError> {
    lock.read().map_err(|e| HypervisorError::RuntimeError(e.to_string()))
}

/// Acquire a `Mutex` lock, mapping poisoning errors to `HypervisorError`.
pub fn mutex_lock<T>(m: &Mutex<T>) -> Result<MutexGuard<T>, HypervisorError> {
    m.lock().map_err(|e| HypervisorError::RuntimeError(e.to_string()))
}

/// Convert an `Option<T>` into a `Result<T, HypervisorError>` with a custom message.
pub fn opt_to_result<T>(opt: Option<T>, msg: &'static str) -> Result<T, HypervisorError> {
    opt.ok_or_else(|| HypervisorError::InvalidData(msg.into()))
}
