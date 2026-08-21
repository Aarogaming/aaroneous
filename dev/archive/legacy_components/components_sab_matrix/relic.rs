// components/agents/src/relic.rs (MCP Edition) - Dual-Stack Design Implementation
  
use serde::{Serialize};
use uuid::Uuid;

/// Import the Capability enum from our new registry.
use crate::sabs::capabilities::{Capability, Equipable}; // TODO: Adjust import path if needed

// ============================================================================  
# THE RELIC STRUCT (Dual-Body Architecture)
// ============================================================================  

/// The Relic Agent - A cybernetic entity capable of wielding multiple capabilities. 
pub struct Relic {
    /// Unique identifier for this agent instance
    pub id: Uuid,
    
    /// Defines the behavioral archetype (e.g., Hephaestus = Forge-focused)  
    pub archetype: Archetype,
    
    /// The equipped tools/capabilities held in memory. 
    // Dual-Stack Design: These are raw Rust structs for native fast-path execution
    pub capabilities: Vec<Capability>,
    
    /// SharedMemory - Neural network state and context (synaptic connections)  
    pub synapse: SharedMemory,
    
    /// MCP Endpoint where this agent exposes its tools to external clients. 
    // None if not serving via MCP protocol; Some(url) when actively listening
    pub mcp_endpoint: Option<String>,
}

/// Archetype defines the behavioral pattern and primary focus of a Relic Agent.  
#[derive(Debug, Clone)]
pub enum Archetype {
    /// Hephaestus - The Forge (Code manipulation specialist) 
    ChimeraForgemaster,
    
    /// Ariel - The Hand (Desktop interaction specialist)  
    MarionetteOperator,
    
    /// Athena - The Eye (Knowledge retrieval specialist)
    OmniOracle,
}

/// SharedMemory represents the neural network state and synaptic connections.  
#[derive(Debug, Clone)]
pub struct SharedMemory {
    pub context: Vec<String>,  // Current working memory/context window 
    pub history: Vec<PhysicsStep>,   # History of physics steps executed
    pub synapses: SynapseMap,        # Neural connection weights (placeholder)
}

/// Represents a single step in the agent's physical execution trace.  
#[derive(Debug, Clone)]
pub struct PhysicsStep {
    pub timestamp: std::time::Instant, 
    pub capability_used: CapabilityKind,  // Which tool was used?
    pub input_data: String,               # Raw data passed to capability
    pub output_result: Option<String>,   # Result of the operation (if any)  
}

/// Enum representing which type of capability is being referenced.  
#[derive(Debug, Clone)]
pub enum CapabilityKind {
    Marionette(MarionetteState), 
    Chimera(ChimeraState),
    Omni(OmniState),
}
