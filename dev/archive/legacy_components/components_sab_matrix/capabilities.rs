// components/skills/src/capabilities.rs
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// The "Organ" System.
/// A standardized enum allowing Relics to hold different types of tools
/// in their SharedMemory slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Capability {
    /// The Hand: OS Interaction & Overlay Control
    Marionette(MarionetteState),

    /// The Forge: Code Manipulation & Compilation
    Chimera(ChimeraState),

    /// The Eye: Data Retrieval & Memory Search
    Omni(OmniState),
}

// ============================================================================
// 1. MARIONETTE (The Hand)
// ============================================================================
/// The internal physics state of the Desktop Interaction capability.
/// The "Glass" (UI) uses this to draw the overlay boxes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarionetteState {
    pub is_active: bool,

    /// The screen coordinates the agent is currently "looking" at.
    /// Format: [x, y, width, height]
    pub focus_bounds: Option<[u32; 4]>,

    /// The specific window/process ID being manipulated.
    pub target_pid: Option<u32>,

    /// Status of the "Hook" (Is the mouse button held? Is input injected?)
    pub hook_status: HookStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HookStatus {
    Idle,
    Scanning,   // "Ghostly outline" mode
    Locked,     // "Solid lock" mode
    Injecting,  // "Active typing/clicking" mode
}

// ============================================================================
// 2. CHIMERA (The Forge)
// ============================================================================
/// The internal physics state of the Code Manipulation capability.
/// The "Glass" uses this to render progress bars and file diffs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChimeraState {
    pub is_active: bool,

    /// The file currently on the anvil.
    pub active_file_path: Option<String>,

    /// 0.0 to 1.0 progress of the current compilation/patch operation.
    pub forge_temperature: f32,

    /// The current operation (e.g., "Decompiling", "Compiling", "Patching")
    pub operation_mode: String,

    /// A list of "Slag" (Errors) produced by the last build.
    pub last_errors: Vec<String>,
}

// ============================================================================
// 3. OMNI (The Eye)
// ============================================================================
/// The internal physics state of the Knowledge/Search capability.
/// The "Glass" uses this to show query results and memory nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniState {
    pub is_active: bool,

    /// The current vector search query.
    pub current_query: String,

    /// The number of memory nodes found.
    pub nodes_found: usize,

    /// The "Confidence" of the retrieval (0.0 - 1.0).
    pub resonance_level: f32,

    /// The raw IDs of the retrieved memories (pointers to vector DB).
    pub memory_pointers: Vec<Uuid>,
}

// ============================================================================
// THE INTERFACE
// ============================================================================
/// Trait for Agents (Relics) that can wield these tools.
pub trait Equipable {
    /// Equips a new capability into the agent's slot.
    fn equip(&mut self, cap: Capability);

    /// Returns the current state of the capability for the UI to render.
    fn inspect_capability(&self) -> Option<&Capability>;
}
