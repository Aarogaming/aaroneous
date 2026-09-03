use anyhow::Result;
use serde::{Serialize, Deserialize};
#[cfg(feature = "llama-gguf")]
use candle_core::CandleBackend; // Optional: for Synthesizer's LLM inference backend

/// The "Machine-Native" Output.
/// Instead of words, agents emit Force Vectors that alter the system state directly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ForceVector {
    pub urgency: f32,             // 0.0 - 1.0 Priority signal
    pub target_hash: u64,         // Hash of the target component/agent
    pub resource_bias: [f32; 4],  // [CPU, GPU, RAM, NET] request vector
    pub action_opcode: u8,        // Raw operation identifier (e.g., 1=ReadMemory, 2=WriteFile)
}

/// The Specialist Roles.
/// Defines the specific biological function of the agent within the hypervisor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Archetype {
    /// ORCHESTRATOR: The High-Frequency Orchestrator.
    /// Manages `clock_tick` and consensus protocols (openraft).
    Orchestrator,

    /// PRESENTER: The Visual Cortex.
    /// Reads `latent_vector` -> Renders `wgpu` / `ratatui` surfaces.
    Presenter,

    /// SYNTHESIZER: The Translator/Intellect.
    /// The ONLY archetype allowed to implement the `Linguist` trait (llama-gguf).
    Synthesizer,

    /// SENTINEL: The Immune System/Sentinel.
    /// Enforces `cap-std` sandboxing and monitors biometric signals (`biometric-ble`).
    Sentinel,

    /// ARCHIVIST: The Metabolizer/Archivist.
    /// Manages RocksDB ingestion (Chaos & Resilience) -> Tantivy indexing for retrieval.
    Archivist,

    /// FABRICATOR: The Stem Cell Factory/WASM Compiler.
    /// Compiles and loads WASM "Frontier Models" into the runtime memory space.
    Fabricator,
}

impl Archetype {
    /// Returns the default clock frequency (Hz) for this archetype's control loop.
    pub fn base_frequency(&self) -> u64 {
        match self {
            Self::Orchestrator => 1000,      // 1kHz Control Loop - Hypervisor heartbeat
            Self::Presenter => 60,       // 60Hz Render Loop - Visual refresh rate (VSync aligned)
            Self::Synthesizer => 5,        // 5-10Hz Cognitive Loop - LLM inference latency bound
            Self::Sentinel => 200,      // 200Hz Security Polling - Threat detection frequency  
            Self::Archivist => 100,   // 100Hz I/O Polling - Database flush cadence
            Self::Fabricator => 5,   // 5-1s Build Loop - WASM compilation latency bound
        }
    }

    /// Returns the archetype's primary resource constraint.
    pub fn primary_resource(&self) -> &'static str {
        match self {
            Archetype::Orchestrator => "CPU",      // Consensus computation intensive  
            Archetype::Presenter => "GPU",     // Rendering and latent space visualization
            Archetype::Synthesizer => "VRAM",   // LLM model weights + KV cache memory
            Archetype::Sentinel => "NET",    // Network monitoring for threat detection
            Archetype::Archivist => "RAM",  // Database storage capacity  
            Archetype::Fabricator => "CPU+GPU", // WASM compilation dual-core requirement
        }
    }

    /// Returns the archetype's primary action opcode range.
    pub fn action_opcode_range(&self) -> std::ops::RangeInclusive<u8> {
        match self {
            Archetype::Orchestrator => 0x10..=0x1F, // 0x10-0x1F: Orchestration ops  
            Archetype::Presenter => 0x20..=0x3F,    // 0x20-0x3F: Rendering/visualization
            Archetype::Synthesizer => 0xA0..=0xAF,  // 0xA0-AF: Linguistic translation ops  
            Archetype::Sentinel => 0xB0..=0xBF,     // 0xB0-BF: Security/sandboxing operations
            Archetype::Archivist => 0xC0..=0xCF,    // 0xC0-CF: Data ingestion/indexing
            Archetype::Fabricator => 0xD0..=0xDF,   // 0xD0-DF: WASM compilation ops  
        }
    }

    /// Returns a human-readable name for the archetype.
    pub fn as_str(&self) -> &'static str {
        match self {
            Archetype::Orchestrator => "ORCHESTRATOR",      // The High-Frequency Orchestrator
            Archetype::Presenter => "PRESENTER",    // The Visual Cortex  
            Archetype::Synthesizer => "SYNTHESIZER",  // The Translator/Intellect
            Archetype::Sentinel => "SENTINEL",   // The Immune System/Sentinel
            Archetype::Archivist => "ARCHIVIST", // The Metabolizer/Archivist
            Archetype::Fabricator => "FABRICATOR",  // The Stem Cell Factory/WASM Compiler  
        }
    }

}


/// The "Silent Core" Trait.
/// MANDATORY: All agents must be able to process raw physics without language.
pub trait NativeThinker {
    /// Reads the current frozen state of the universe (SharedMemory)
    /// and calculates a deterministic reaction (ForceVector).
    fn process_physics(&self, state: &nervous_system::shared_memory::SynapseState) -> Result<ForceVector>;

    /// Returns the agent's unique biological signature.
    fn archetype(&self) -> Archetype;

}


/// The "Language Skill" Trait.
/// OPTIONAL: Only implemented by Synthesizer or User Proxies (UserAgent).
#[cfg(feature = "llama-gguf")]
pub trait Linguist {
    /// Converts a ForceVector back into human-readable text using LLM inference.
    fn translate_intent(&self, vector: &ForceVector) -> Result<String>;

    /// Ingests human text and compresses it into the SharedMemory latent_vector space (1024-dim).
    fn embed_concept(&self, text: &str, state: &mut nervous_system::shared_memory::SynapseState) -> Result<()>;


}

/// Fabricator-Specific: The Frontier Loader.
/// This defines how the Builder agent injects new skills (WASM modules) into the runtime memory space.
pub trait FrontierLoader {
    /// Hot-loads a WASM blob ("Frontier Model") into a Specialist's execution context at runtime.
    fn inject_skill(&self, agent_id: u64, wasm_path: std::path::PathBuf) -> Result<()>;

}


// ============================================================================
// DEFAULT IMPLEMENTATION STUBS (The "Reflexes" for each archetype)  
// These provide baseline physics processing behavior that can be overridden.
// ============================================================================

impl NativeThinker for Archetype {
    /// Default implementation: Returns a null force vector with minimal urgency.
    fn process_physics(&self, _state: &nervous_system::shared_memory::SynapseState) -> Result<ForceVector> {
        Ok(ForceVector {
            urgency: 0.1f32.min(self.base_frequency() as f32 / 5000.0), // Scale by frequency  
            target_hash: self.hash(),                                    // Hash of archetype itself (self-reference)
            resource_bias: [0.0; 4],                                     // No specific resource request yet
            action_opcode: *Self::action_opcode_range(self).start(),     // Use minimum opcode for this type  
        })
    }

    fn archetype(&self) -> Archetype {
        self.clone()
    }
}


impl Archetype {
    /// Returns a hash of the archetype (used in SharedMemory.target_hash field)
    pub fn hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        Hash::hash(self, &mut hasher);
        hasher.finish()
    }

}

// ============================================================================
// ARCHETYPE-SPECIFIC IMPLEMENTATIONS
// These would live in separate files like archetypes/orchestrator.rs, archetypes/presenter.rs, etc.
// For now, we provide the trait definitions and default implementations above.
// ============================================================================

// Synthesizer-specific implementation goes in a separate file when llama-gguf feature is enabled.
// Fabricator-specific WASM loading logic would go here (using wasmtime/wasi).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archetype_properties_and_signatures() {
        let archetypes = [
            Archetype::Orchestrator,
            Archetype::Presenter,
            Archetype::Synthesizer,
            Archetype::Sentinel,
            Archetype::Archivist,
            Archetype::Fabricator,
        ];

        for a in &archetypes {
            assert!(a.base_frequency() > 0);
            assert!(!a.primary_resource().is_empty());
            assert!(!a.as_str().is_empty());
            assert!(a.hash() > 0);
            assert_eq!(a.archetype(), a.clone());
        }

        assert_eq!(Archetype::Orchestrator.base_frequency(), 1000);
        assert_eq!(Archetype::Presenter.base_frequency(), 60);
        assert_eq!(Archetype::Synthesizer.base_frequency(), 5);
        assert_eq!(Archetype::Sentinel.base_frequency(), 200);
        assert_eq!(Archetype::Archivist.base_frequency(), 100);
        assert_eq!(Archetype::Fabricator.base_frequency(), 5);
    }

    #[test]
    fn test_archetype_opcode_ranges_are_disjoint() {
        let r1 = Archetype::action_opcode_range(&Archetype::Orchestrator);
        let r2 = Archetype::action_opcode_range(&Archetype::Presenter);
        let r3 = Archetype::action_opcode_range(&Archetype::Synthesizer);
        let r4 = Archetype::action_opcode_range(&Archetype::Sentinel);
        let r5 = Archetype::action_opcode_range(&Archetype::Archivist);
        let r6 = Archetype::action_opcode_range(&Archetype::Fabricator);

        assert_eq!(r1, 0x10..=0x1F);
        assert_eq!(r2, 0x20..=0x3F);
        assert_eq!(r3, 0xA0..=0xAF);
        assert_eq!(r4, 0xB0..=0xBF);
        assert_eq!(r5, 0xC0..=0xCF);
        assert_eq!(r6, 0xD0..=0xDF);
    }

    #[test]
    fn test_force_vector_serialization() {
        let fv = ForceVector {
            urgency: 0.75,
            target_hash: 123456789,
            resource_bias: [1.0, 0.5, 0.2, 0.1],
            action_opcode: 0x81,
        };

        let json = serde_json::to_string(&fv).unwrap();
        let restored: ForceVector = serde_json::from_str(&json).unwrap();
        assert!((restored.urgency - 0.75).abs() < 1e-6);
        assert_eq!(restored.target_hash, 123456789);
        assert_eq!(restored.action_opcode, 0x81);
    }
}

