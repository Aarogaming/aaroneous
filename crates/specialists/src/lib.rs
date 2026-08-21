//! crates/specialists
//! The Specialist Federation of 9 Sovereign Domain Engines and their paired Relic Substrates for Aaroneous.

pub mod argus;
pub mod ariel;
pub mod dionysus;
pub mod hephaestus;
pub mod hermes;
pub mod kami;
pub mod merlin;
pub mod odin;
pub mod traits;
pub mod wen;

pub use argus::{ArgusSpecialist, SecurityAuditReport, SentinelRelic};
pub use ariel::{ArielSpecialist, GlassRelic, UiPresentationFrame};
pub use dionysus::{DionysusSpecialist, OmniRelicWrapper};
pub use hephaestus::{ForgeRelic, HephaestusSpecialist};
pub use hermes::{CaduceusRelic, HermesSpecialist, MeshPeerState};
pub use kami::{KamiSpecialist, ThresholdRelicWrapper};
pub use merlin::{GrimoireRelic, KnowledgeSynthesis, MerlinSpecialist};
pub use odin::{DraupnirRelic, OdinSpecialist, TaskNode};
pub use traits::{MnlpPacket, MnlpResponse, RelicEngine, SovereignSpecialist, SpecialistHealth};
pub use wen::{ResonanceRelic, SymbioticResonanceReport, WenSpecialist};

use anyhow::{bail, Result};
use std::collections::HashMap;

/// The Unified Specialist Federation managing all 9 Sovereign Domain Specialists
pub struct SpecialistFederation {
    pub odin: OdinSpecialist,
    pub merlin: MerlinSpecialist,
    pub ariel: ArielSpecialist,
    pub hephaestus: HephaestusSpecialist,
    pub argus: ArgusSpecialist,
    pub dionysus: DionysusSpecialist,
    pub hermes: HermesSpecialist,
    pub wen: WenSpecialist,
    pub kami: KamiSpecialist,
}

/// Backwards-compatible alias for the Specialist Federation
pub type OlympianPantheon = SpecialistFederation;

impl Default for SpecialistFederation {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecialistFederation {
    pub fn new() -> Self {
        Self {
            odin: OdinSpecialist::new(),
            merlin: MerlinSpecialist::new(),
            ariel: ArielSpecialist::new(),
            hephaestus: HephaestusSpecialist::new(),
            argus: ArgusSpecialist::new(),
            dionysus: DionysusSpecialist::new(),
            hermes: HermesSpecialist::new(),
            wen: WenSpecialist::new(),
            kami: KamiSpecialist::new(),
        }
    }

    /// Dispatches a machine-native packet to the appropriate sovereign specialist by opcode
    pub async fn dispatch_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse> {
        match packet.opcode {
            0x0100 => self.odin.handle_packet(packet).await,
            0x0200 => self.merlin.handle_packet(packet).await,
            0x0300 => self.ariel.handle_packet(packet).await,
            0x0400 => self.hephaestus.handle_packet(packet).await,
            0x0500 => self.argus.handle_packet(packet).await,
            0x0600 => self.dionysus.handle_packet(packet).await,
            0x0700 => self.hermes.handle_packet(packet).await,
            0x0800 => self.wen.handle_packet(packet).await,
            0x0900 => self.kami.handle_packet(packet).await,
            _ => bail!("Unknown domain opcode: 0x{:04X}", packet.opcode),
        }
    }

    /// Collects health reports across the entire pantheon
    pub fn collect_health_reports(&self) -> HashMap<&'static str, SpecialistHealth> {
        let mut reports = HashMap::new();
        reports.insert(self.odin.name(), self.odin.health_report());
        reports.insert(self.merlin.name(), self.merlin.health_report());
        reports.insert(self.ariel.name(), self.ariel.health_report());
        reports.insert(self.hephaestus.name(), self.hephaestus.health_report());
        reports.insert(self.argus.name(), self.argus.health_report());
        reports.insert(self.dionysus.name(), self.dionysus.health_report());
        reports.insert(self.hermes.name(), self.hermes.health_report());
        reports.insert(self.wen.name(), self.wen.health_report());
        reports.insert(self.kami.name(), self.kami.health_report());
        reports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_specialist_federation_dispatch() {
        let mut federation = SpecialistFederation::new();

        // 1. Test Odin (0x0100)
        let pkt_odin = MnlpPacket {
            opcode: 0x0100,
            source: "user".to_string(),
            target: "odin".to_string(),
            correlation_id: 1,
            payload: b"Deconstruct target app".to_vec(),
        };
        let res_odin = federation.dispatch_packet(pkt_odin).await.unwrap();
        assert!(res_odin.success);

        // 2. Test Merlin (0x0200)
        let pkt_merlin = MnlpPacket {
            opcode: 0x0200,
            source: "user".to_string(),
            target: "merlin".to_string(),
            correlation_id: 2,
            payload: b"Rust FFI linking".to_vec(),
        };
        let res_merlin = federation.dispatch_packet(pkt_merlin).await.unwrap();
        assert!(res_merlin.success);

        // 3. Test Argus (0x0500)
        let pkt_argus = MnlpPacket {
            opcode: 0x0500,
            source: "user".to_string(),
            target: "argus".to_string(),
            correlation_id: 3,
            payload: b"Safe binary verification".to_vec(),
        };
        let res_argus = federation.dispatch_packet(pkt_argus).await.unwrap();
        assert!(res_argus.success);

        // 4. Test Health collection across all 9 specialists
        let health = federation.collect_health_reports();
        assert_eq!(health.len(), 9);
        assert!(health.contains_key("Odin"));
        assert!(health.contains_key("Merlin"));
        assert!(health.contains_key("Ariel"));
        assert!(health.contains_key("Hephaestus"));
        assert!(health.contains_key("Argus"));
        assert!(health.contains_key("Dionysus"));
        assert!(health.contains_key("Hermes"));
        assert!(health.contains_key("Wen"));
        assert!(health.contains_key("Kami"));
    }
}
