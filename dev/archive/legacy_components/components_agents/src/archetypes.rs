use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
#[cfg(feature = "llama-gguf")]
use candle_core::CandleBackend; // Optional: for Merlin's LLM inference backend

/// The "Machine-Native" Output.
/// Instead of words, agents emit Force Vectors that alter the system state directly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ForceVector {
    pub urgency: f32,             // 0.0 - 1.0 Priority signal
    pub target_hash: u64,         // Hash of the target component/agent
    pub resource_bias: [f32; 4],  // [CPU, GPU, RAM, NET] request vector
    pub action_opcode: u8,        // Raw operation identifier (e.g., 1=ReadMemory, 2=WriteFile)
}

/// The Pantheon Roles.
/// Defines the specific biological function of the agent within the hypervisor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Archetype {
    /// ODIN: The High-Frequency Orchestrator.
    /// Manages `clock_tick` and consensus protocols (openraft).
    Odin,

    /// ARIEL: The Visual Cortex.
    /// Reads `latent_vector` -> Renders `wgpu` / `ratatui` surfaces.
    Ariel,

    /// MERLIN: The Translator/Intellect.
    /// The ONLY archetype allowed to implement the `Linguist` trait (llama-gguf).
    Merlin,

    /// ARGUS: The Immune System/Sentinel.
    /// Enforces `cap-std` sandboxing and monitors biometric signals (`biometric-ble`).
    Argus,

    /// DIONYSUS: The Metabolizer/Archivist.
    /// Manages RocksDB ingestion (Dionysian chaos) -> Tantivy indexing for retrieval.
    Dionysus,

    /// HEPHAESTUS: The Stem Cell Factory/WASM Compiler.
    /// Compiles and loads WASM "Frontier Models" into the runtime memory space.
    Hephaestus,
}

impl Archetype {
    /// Returns the default clock frequency (Hz) for this archetype's control loop.
    pub fn base_frequency(&self) -> u64 {
        match self {
            Self::Odin => 1000,      // 1kHz Control Loop - Hypervisor heartbeat
            Self::Ariel => 60,       // 60Hz Render Loop - Visual refresh rate (VSync aligned)
            Self::Merlin => 5,        // 5-10Hz Cognitive Loop - LLM inference latency bound
            Self::Argus => 200,      // 200Hz Security Polling - Threat detection frequency  
            Self::Dionysus => 100,   // 100Hz I/O Polling - Database flush cadence
            Self::Hephaestus => 5,   // 5-1s Build Loop - WASM compilation latency bound
        }
    }

    /// Returns the archetype's primary resource constraint.
    pub fn primary_resource(&self) -> &'static str {
        match self {
            Archetype::Odin => "CPU",      // Consensus computation intensive  
            Archetype::Ariel => "GPU",     // Rendering and latent space visualization
            Archetype::Merlin => "VRAM",   # LLM model weights + KV cache memory
            Archetype::Argus => "NET",    // Network monitoring for threat detection
            Archetype::Dionysus => "RAM",  // Database storage capacity  
            Archetype::Hephaestus => "CPU+GPU", // WASM compilation dual-core requirement
        }
    }

    /// Returns the archetype's primary action opcode range.
    pub fn action_opcode_range(&self) -> (u8, u8) {
        match self {
            Archetype::Odin => (0x10..=0x1F),   // 0x10-0x1F: Orchestration ops  
            Archetype::Ariel => (0x20..=0x3F),  # 0x20-0x3F: Rendering/visualization
            Archetype::Merlin => (0xA0..=0xAF), // 0xA0-AF: Linguistic translation ops  
            Archetype::Argus => (0xB0..=0xBF),  // 0xB0-BF: Security/sandboxing operations
            Archetype::Dionysus => (0xC0..=0xCF), # 0xC0-CF: Data ingestion/indexing
            Archetype::Hephaestus => (0xD0..=0xDF), // 0xD0-DF: WASM compilation ops  
        }
    }

    /// Returns a human-readable name for the archetype.
    pub fn as_str(&self) -> &'static str {
        match self {
            Archetype::Odin => "ODIN",      # The High-Frequency Orchestrator
            Archetype::Ariel => "ARIEL",    // The Visual Cortex  
            Archetype::Merlin => "MERLIN",  // The Translator/Intellect
            Archetype::Argus => "ARGUS",   // The Immune System/Sentinel
            Archetype::Dionysus => "DIONYSUS", # The Metabolizer/Archivist
            Archetype::Hephaestus => "HEPHAESTUS",  // The Stem Cell Factory/WASM Compiler  
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
/// OPTIONAL: Only implemented by Merlin or User Proxies (UserAgent).
#[cfg(feature = "llama-gguf")]
pub trait Linguist {
    /// Converts a ForceVector back into human-readable text using LLM inference.
    fn translate_intent(&self, vector: &ForceVector) -> Result<String>;

    /// Ingests human text and compresses it into the SharedMemory latent_vector space (1024-dim).
    fn embed_concept(&self, text: &str, state: &mut nervous_system::shared_memory::SynapseState) -> Result<()>;


}

/// Hephaestus-Specific: The Frontier Loader.
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
            target_hash: self.hash(),                                    # Hash of archetype itself (self-reference)
            resource_bias: [0.0; 4],                                     # No specific resource request yet
            action_opcode: *Self::action_opcode_range(self).start() as u8,   # Use minimum opcode for this type  
        })
    }

}


impl Archetype {
    /// Returns a hash of the archetype (used in SharedMemory.target_hash field)
    pub fn hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

}


// ============================================================================
// ARCHETYPE-SPECIFIC IMPLEMENTATIONS (To be filled by individual agent modules)  
// These would live in separate files like archetypes/odin.rs, archetypes/ariel.rs, etc.
// For now, we provide the trait definitions and default implementations above.
// ============================================================================

/// Odin-specific implementation stubs go here:
impl NativeThinker for Archetype {
    /// Override this to implement consensus protocol logic (openraft).  
}


/// Ariel-specific implementation stubs go here:
#[cfg(feature = "wgpu")]
impl NativeThinker for Archetype {
    /// Override this to render latent vectors into TUI surfaces.  
}

// Merlin-specific implementation goes in a separate file when llama-gguf feature is enabled.
// Hephaestus-specific WASM loading logic would go here (using wasmtime/wasi).

