//! crates/specialists
//! The Specialist Federation of 9 Sovereign Domain Engines and their paired Relic Substrates for Aaroneous.

pub mod aligner;
pub mod archivist;
pub mod code_specialist;
pub mod dev_tools;
pub mod orchestrator;
pub mod perceiver;
pub mod presenter;
pub mod router;
pub mod sentinel;
pub mod synthesizer;
pub mod traits;

pub extern crate ipc_bus as nervous_system;
pub use ipc_bus;
pub extern crate autonomic_adaptation as evolution;
pub use autonomic_adaptation;

pub use aligner::{AlignerSpecialist, AlignmentEngine, HarmonyEngineRelic, SymbioticResonanceReport};
pub use archivist::{ArchivistSpecialist, MemoryIndexEngine, MemoryIndexRelic};
pub use dev_tools::{CompilerCoreRelic, CompilerForgeEngine, DevToolsSpecialist, FabricatorSpecialist};
pub use orchestrator::{OrchestratorCoreRelic, OrchestratorSpecialist, TaskNode, TaskSchedulerEngine};
pub use perceiver::{GatekeeperEngineRelic, PerceptionGateEngine, PerceiverSpecialist};
pub use presenter::{DisplayBufferEngine, DisplayBufferRelic, PresenterSpecialist, UiPresentationFrame};
pub use router::{FederationBusRelic, MeshPeerState, MeshRouterEngine, RouterSpecialist};
pub use sentinel::{AuditEngineRelic, SecurityAuditEngine, SecurityAuditReport, SentinelSpecialist};
pub use synthesizer::{KnowledgeStoreEngine, KnowledgeStoreRelic, KnowledgeSynthesis, SynthesizerSpecialist};
pub use traits::{DomainSubEngine, MnlpPacket, MnlpResponse, RelicEngine, Specialist, SovereignSpecialist, SpecialistHealth};

use anyhow::{bail, Result};
use std::collections::HashMap;

/// The Unified Specialist Federation / Pool managing all 9 Specialists
pub struct SpecialistFederation {
    pub orchestrator: OrchestratorSpecialist,
    pub synthesizer: SynthesizerSpecialist,
    pub presenter: PresenterSpecialist,
    pub dev_tools: DevToolsSpecialist,
    pub sentinel: SentinelSpecialist,
    pub archivist: ArchivistSpecialist,
    pub router: RouterSpecialist,
    pub aligner: AlignerSpecialist,
    pub perceiver: PerceiverSpecialist,
}

/// Simplified alias for the Specialist Federation
pub type Specialists = SpecialistFederation;

/// Hub alias for the Specialist Federation
pub type SpecialistHub = SpecialistFederation;

/// Backwards-compatible alias for the Specialist Federation
pub type SpecialistFederationAlias = SpecialistFederation;

impl Default for SpecialistFederation {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecialistFederation {
    pub fn new() -> Self {
        Self {
            orchestrator: OrchestratorSpecialist::new(),
            synthesizer: SynthesizerSpecialist::new(),
            presenter: PresenterSpecialist::new(),
            dev_tools: DevToolsSpecialist::new(),
            sentinel: SentinelSpecialist::new(),
            archivist: ArchivistSpecialist::new(),
            router: RouterSpecialist::new(),
            aligner: AlignerSpecialist::new(),
            perceiver: PerceiverSpecialist::new(),
        }
    }

    /// Dispatches a machine-native packet to the appropriate sovereign specialist by opcode
    pub async fn dispatch_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse> {
        match packet.opcode {
            0x0100 => self.orchestrator.handle_packet(packet).await,
            0x0200 => self.synthesizer.handle_packet(packet).await,
            0x0300 => self.presenter.handle_packet(packet).await,
            0x0400 => self.dev_tools.handle_packet(packet).await,
            0x0500 => self.sentinel.handle_packet(packet).await,
            0x0600 => self.archivist.handle_packet(packet).await,
            0x0700 => self.router.handle_packet(packet).await,
            0x0800 => self.aligner.handle_packet(packet).await,
            0x0900 => self.perceiver.handle_packet(packet).await,
            _ => bail!("Unknown domain opcode: 0x{:04X}", packet.opcode),
        }
    }

    /// Collects health reports across the entire specialist federation
    pub fn collect_health_reports(&self) -> HashMap<&'static str, SpecialistHealth> {
        let mut reports = HashMap::new();
        reports.insert(self.orchestrator.name(), self.orchestrator.health_report());
        reports.insert(self.synthesizer.name(), self.synthesizer.health_report());
        reports.insert(self.presenter.name(), self.presenter.health_report());
        reports.insert(self.dev_tools.name(), self.dev_tools.health_report());
        reports.insert(self.sentinel.name(), self.sentinel.health_report());
        reports.insert(self.archivist.name(), self.archivist.health_report());
        reports.insert(self.router.name(), self.router.health_report());
        reports.insert(self.aligner.name(), self.aligner.health_report());
        reports.insert(self.perceiver.name(), self.perceiver.health_report());
        reports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_specialist_federation_dispatch() {
        let mut federation = SpecialistFederation::new();

        // 1. Test Orchestrator (0x0100)
        let pkt_orchestrator = MnlpPacket {
            opcode: 0x0100,
            source: "user".to_string(),
            target: "Orchestrator".to_string(),
            correlation_id: 1,
            payload: b"Deconstruct target app".to_vec(),
        };
        let res_orchestrator = federation.dispatch_packet(pkt_orchestrator).await.unwrap();
        assert!(res_orchestrator.success);

        // 2. Test Synthesizer (0x0200)
        let pkt_synthesizer = MnlpPacket {
            opcode: 0x0200,
            source: "user".to_string(),
            target: "Synthesizer".to_string(),
            correlation_id: 2,
            payload: b"Rust FFI linking".to_vec(),
        };
        let res_synthesizer = federation.dispatch_packet(pkt_synthesizer).await.unwrap();
        assert!(res_synthesizer.success);

        // 3. Test Sentinel (0x0500)
        let pkt_sentinel = MnlpPacket {
            opcode: 0x0500,
            source: "user".to_string(),
            target: "Sentinel".to_string(),
            correlation_id: 3,
            payload: b"Safe binary verification".to_vec(),
        };
        let res_sentinel = federation.dispatch_packet(pkt_sentinel).await.unwrap();
        assert!(res_sentinel.success);

        // 4. Test Health collection across all 9 specialists
        let health = federation.collect_health_reports();
        assert_eq!(health.len(), 9);
        assert!(health.contains_key("Orchestrator"));
        assert!(health.contains_key("Synthesizer"));
        assert!(health.contains_key("Presenter"));
        assert!(health.contains_key("Fabricator"));
        assert!(health.contains_key("Sentinel"));
        assert!(health.contains_key("Archivist"));
        assert!(health.contains_key("Router"));
        assert!(health.contains_key("Aligner"));
        assert!(health.contains_key("Perceiver"));
    }
}
