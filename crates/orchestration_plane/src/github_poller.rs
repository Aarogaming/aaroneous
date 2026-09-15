//! GitHub Poller Kernel — Decoupled Logic Controller Implementation
//! Zero dynamic allocations, no ambient authority, no unsafe pointer mutations.

#![forbid(unsafe_code)]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HandlerError(pub u32);

impl HandlerError {
    pub const QUEUE_FULL: Self = Self(1);
    pub const INVALID_PAYLOAD: Self = Self(2);
    pub const EXECUTION_FAILED: Self = Self(3);
    pub const CIRCUIT_TRIPPED: Self = Self(4);
}

impl core::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::QUEUE_FULL => write!(f, "queue_full"),
            Self::INVALID_PAYLOAD => write!(f, "invalid_payload"),
            Self::EXECUTION_FAILED => write!(f, "execution_failed"),
            Self::CIRCUIT_TRIPPED => write!(f, "circuit_tripped"),
            _ => write!(f, "unknown_error({})", self.0),
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct JobStatus(pub u8);

impl JobStatus {
    pub const EMPTY: Self = Self(0);
    pub const QUEUED: Self = Self(1);
    pub const PROCESSING: Self = Self(2);
    pub const COMPLETE: Self = Self(3);
    pub const ERROR: Self = Self(4);
    pub const DEDUPLICATED: Self = Self(5);
}

impl core::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::EMPTY => write!(f, "empty"),
            Self::QUEUED => write!(f, "queued"),
            Self::PROCESSING => write!(f, "processing"),
            Self::COMPLETE => write!(f, "complete"),
            Self::ERROR => write!(f, "error"),
            Self::DEDUPLICATED => write!(f, "deduplicated"),
            _ => write!(f, "unknown({})", self.0),
        }
    }
}

pub type Sha1 = [u8; 40];

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Job {
    pub id: [u8; 32],
    pub source_sha: Sha1,
    pub status: JobStatus,
    pub _pad: [u8; 7],
    pub created_ns: u64,
    pub updated_ns: u64,
    pub reserved: [u8; 32],
}

impl Job {
    pub const fn empty() -> Self {
        Self {
            id: [0u8; 32],
            source_sha: [0u8; 40],
            status: JobStatus::EMPTY,
            _pad: [0u8; 7],
            created_ns: 0,
            updated_ns: 0,
            reserved: [0u8; 32],
        }
    }

    pub const fn new(id: [u8; 32], source_sha: Sha1, now_ns: u64) -> Self {
        Self {
            id,
            source_sha,
            status: JobStatus::QUEUED,
            _pad: [0u8; 7],
            created_ns: now_ns,
            updated_ns: now_ns,
            reserved: [0u8; 32],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PollerConfig {
    pub base_interval_ns: u64,
    pub min_interval_ns: u64,
    pub max_retries: u32,
    pub _pad: u32,
}

impl PollerConfig {
    pub const fn new(interval_seconds: u64) -> Self {
        let base_ns = interval_seconds * 1_000_000_000;
        Self {
            base_interval_ns: base_ns,
            min_interval_ns: (base_ns / 100) * 85,
            max_retries: 3,
            _pad: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PollerStats {
    pub capacity: u64,
    pub len: u64,
    pub base_interval_ns: u64,
    pub running: bool,
    pub _pad: [u8; 7],
}

pub trait JobHandler {
    fn process(&self, job: &Job) -> Result<(), HandlerError>;
}

pub trait ShaProvider {
    fn fetch_head_sha(&self) -> Option<Sha1>;
}

pub struct GitHubPoller<const CAPACITY: usize> {
    config: PollerConfig,
    running: AtomicBool,
    head: AtomicU64,
    tail: AtomicU64,
    start_time: Instant,
}

impl<const CAPACITY: usize> GitHubPoller<CAPACITY> {
    pub fn new(config: PollerConfig) -> Self {
        Self {
            config,
            running: AtomicBool::new(false),
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    pub fn current_epoch_ns(&self) -> u64 {
        self.start_time.elapsed().as_nanos() as u64
    }

    pub fn delay_with_jitter(&self, base_ns: u64) {
        if base_ns == 0 {
            return;
        }
        let min_ns = (base_ns / 100) * 85;
        let start = Instant::now();
        while (start.elapsed().as_nanos() as u64) < min_ns {
            core::hint::spin_loop();
        }
    }

    pub fn poll_and_dispatch<P, H>(&self, provider: &P, handler: &H) -> Result<bool, HandlerError>
    where
        P: ShaProvider,
        H: JobHandler,
    {
        if !self.is_running() {
            return Ok(false);
        }

        let sha = match provider.fetch_head_sha() {
            Some(s) => s,
            None => return Ok(false),
        };

        let now_ns = self.current_epoch_ns();
        let mut id = [0u8; 32];
        id[0..8].copy_from_slice(&now_ns.to_le_bytes());
        id[8..16].copy_from_slice(&self.tail.load(Ordering::Relaxed).to_le_bytes());

        let job = Job::new(id, sha, now_ns);

        handler.process(&job)?;
        self.tail.fetch_add(1, Ordering::Release);

        Ok(true)
    }

    pub fn stats(&self) -> PollerStats {
        let t = self.tail.load(Ordering::Acquire);
        let h = self.head.load(Ordering::Acquire);
        PollerStats {
            capacity: CAPACITY as u64,
            len: t.saturating_sub(h),
            base_interval_ns: self.config.base_interval_ns,
            running: self.is_running(),
            _pad: [0u8; 7],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_pod_geometry_and_bitcast() {
        assert_eq!(core::mem::size_of::<Job>(), 128);
        assert_eq!(core::mem::align_of::<Job>(), 8);

        let job = Job::empty();
        let bytes: &[u8] = bytemuck::bytes_of(&job);
        assert_eq!(bytes.len(), 128);

        let restored: &Job = bytemuck::from_bytes(bytes);
        assert_eq!(restored.status, JobStatus::EMPTY);
        assert_eq!(restored.created_ns, 0);
    }

    #[test]
    fn test_poller_config_pod_geometry() {
        assert_eq!(core::mem::size_of::<PollerConfig>(), 24);
        let config = PollerConfig::new(30);
        let bytes = bytemuck::bytes_of(&config);
        let restored: &PollerConfig = bytemuck::from_bytes(bytes);
        assert_eq!(restored.base_interval_ns, 30_000_000_000);
        assert_eq!(restored.max_retries, 3);
    }

    #[test]
    fn test_lifecycle_and_atomic_counters() {
        let config = PollerConfig::new(60);
        let poller = GitHubPoller::<1024>::new(config);

        assert!(!poller.is_running());
        poller.start();
        assert!(poller.is_running());

        let stats = poller.stats();
        assert_eq!(stats.capacity, 1024);
        assert_eq!(stats.len, 0);
        assert!(stats.running);

        poller.stop();
        assert!(!poller.is_running());
    }
}
