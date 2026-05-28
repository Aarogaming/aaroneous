use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct BackgroundTaskHandle {
    pub name: String,
    running: Arc<AtomicBool>,
}

impl BackgroundTaskHandle {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            running: Arc::new(AtomicBool::new(true)),
        }
    }
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    pub async fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
    pub async fn shutdown(&mut self) {
        self.stop().await
    }
}

pub struct OmnipresentRecvTask;
impl OmnipresentRecvTask {
    pub fn new() -> Self { Self }
    pub fn start(&self) -> BackgroundTaskHandle { BackgroundTaskHandle::new("omnipresent_recv") }
}

pub struct SymbioticBleTask;
impl SymbioticBleTask {
    pub fn new() -> Self { Self }
    pub fn start(&self) -> BackgroundTaskHandle { BackgroundTaskHandle::new("symbiotic_ble") }
}

pub struct OmnipresentDrainTask;
impl OmnipresentDrainTask {
    pub fn new() -> Self { Self }
    pub fn start(&self) -> BackgroundTaskHandle { BackgroundTaskHandle::new("omnipresent_drain") }
}

pub struct SymbioticDrainTask;
impl SymbioticDrainTask {
    pub fn new() -> Self { Self }
    pub fn start(&self) -> BackgroundTaskHandle { BackgroundTaskHandle::new("symbiotic_drain") }
}
