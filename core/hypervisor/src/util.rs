// src/util.rs
//! Utility helpers for safe lock acquisition and error mapping in the hypervisor ACC.

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::error::HypervisorError;

/// Acquire a write lock on an `RwLock`, mapping any poisoning error to `HypervisorError`.
pub fn lock_write<T>(lock: &RwLock<T>) -> Result<RwLockWriteGuard<T>, HypervisorError> {
    let err_msg = "Failed to acquire write lock";
    match std::str::from_utf8(err_msg.as_bytes()) {
        Ok(s) => lock.write().map_err(|e| HypervisorError::RuntimeError(s.to_string())),
        Err(_) => lock.write().map_err(|e| HypervisorError::RuntimeError(err_msg.to_string())),
    }
}

/// Acquire a read lock on an `RwLock`, mapping any poisoning error to `HypervisorError`.
pub fn lock_read<T>(lock: &RwLock<T>) -> Result<RwLockReadGuard<T>, HypervisorError> {
    let err_msg = "Failed to acquire read lock";
    match std::str::from_utf8(err_msg.as_bytes()) {
        Ok(s) => lock.read().map_err(|e| HypervisorError::RuntimeError(s.to_string())),
        Err(_) => lock.read().map_err(|e| HypervisorError::RuntimeError(err_msg.to_string())),
    }
}

/// Acquire a `Mutex` lock, mapping poisoning errors to `HypervisorError`.
pub fn mutex_lock<T>(m: &Mutex<T>) -> Result<MutexGuard<T>, HypervisorError> {
    let err_msg = "Failed to acquire mutex lock";
    match std::str::from_utf8(err_msg.as_bytes()) {
        Ok(s) => m.lock().map_err(|e| HypervisorError::RuntimeError(s.to_string())),
        Err(_) => m.lock().map_err(|e| HypervisorError::RuntimeError(err_msg.to_string())),
    }
}

/// Convert an `Option<T>` into a `Result<T, HypervisorError>` with a custom message.
pub fn opt_to_result<T>(opt: Option<T>, msg: &'static str) -> Result<T, HypervisorError> {
    opt.ok_or_else(|| HypervisorError::InvalidData(msg.to_string()))
}
