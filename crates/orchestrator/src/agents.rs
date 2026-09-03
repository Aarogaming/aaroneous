// Aaroneous Agent Taxonomy Module
// Defines core Agent, Specialist, Relic, and User classes with trait-based composition

use crate::workspace::WorkspacePaths;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use strum::{Display, EnumIter, EnumString};

/// Base Agent trait - all sentient entities in the hive implement this
pub trait Agent: Send + Sync {
    fn agent_id(&self) -> &str;
    fn agent_type(&self) -> AgentType;
    fn get_persona(&self) -> &str;
    fn get_cognitive_bias(&self) -> &CognitiveBias;
    fn get_role(&self) -> &str;
}

/// Core agent types in the Aaroneous hive
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Display, EnumString, EnumIter,
)]
pub enum AgentType {
    #[strum(serialize = "BaseAgent")]
    BaseAgent, // Aaroneous (Agent-Zero)
    #[strum(serialize = "Specialist")]
    Specialist, // Interactive personifications (Presenter, Synthesizer, Orchestrator, etc.)
    #[strum(serialize = "Relic")]
    Relic, // Smart artifacts supervised by specialists
    #[strum(serialize = "User")]
    User, // Human end-users (interactive via system)
}

/// Cognitive bias profile for personalized reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveBias {
    pub analytical_depth: u32,  // 0-100: How deeply to analyze
    pub creative_variance: u32, // 0-100: How creative/exploratory
    pub audit_strictness: u32,  // 0-100: How strict about validation
}

impl Default for CognitiveBias {
    fn default() -> Self {
        CognitiveBias {
            analytical_depth: 75,
            creative_variance: 50,
            audit_strictness: 50,
        }
    }
}

/// Domain specialty for agent focus
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Display, EnumString, EnumIter,
)]
pub enum Domain {
    #[strum(serialize = "UserInterface")]
    UserInterface, // Presenter
    #[strum(serialize = "Knowledge")]
    Knowledge, // Synthesizer
    #[strum(serialize = "Leadership")]
    Leadership, // Orchestrator
    #[strum(serialize = "Experience")]
    Experience, // Circe
    #[strum(serialize = "Manufacturing")]
    Manufacturing, // Fabricator
    #[strum(serialize = "Security")]
    Security, // Sentinel
    #[strum(serialize = "Undefined")]
    Undefined, // For base agents
}

/// Specialist type - interactive personification that supervises relics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistAgent {
    pub id: String,
    pub name: String, // "Presenter", "Synthesizer", "Orchestrator", etc.
    pub domain: Domain,
    pub role: String,    // "UI Designer", "Knowledge Synthesist", etc.
    pub persona: String, // Personality flavor text
    pub cognitive_bias: CognitiveBias,
    pub supervised_relic: Option<String>, // ID of supervised relic
    pub hox_preset_path: String,          // Path to hox_specialist_<name>.json
    pub enzyme_subset: Vec<String>,       // Allowlisted enzymes for this specialist
    pub interval_ms: u64,                 // Polling interval in milliseconds
    pub model_path: String,               // Path to GGUF model file
    pub model_hash: String,               // SHA256 hash of model file
    pub status: String,                   // Current operational status
}

impl Agent for SpecialistAgent {
    fn agent_id(&self) -> &str {
        &self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Specialist
    }

    fn get_persona(&self) -> &str {
        &self.persona
    }

    fn get_cognitive_bias(&self) -> &CognitiveBias {
        &self.cognitive_bias
    }

    fn get_role(&self) -> &str {
        &self.role
    }
}

impl Default for SpecialistAgent {
    fn default() -> Self {
        let paths = WorkspacePaths::discover();
        SpecialistAgent {
            id: "specialist_default".to_string(),
            name: "Template".to_string(),
            domain: Domain::Undefined,
            role: "Observer".to_string(),
            persona: "A neutral entity observing the hive.".to_string(),
            cognitive_bias: CognitiveBias::default(),
            supervised_relic: None,
            hox_preset_path: paths
                .registry()
                .join("hox_specialist_template.json")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec![],
            interval_ms: 30000,
            model_path: String::new(),
            model_hash: String::new(),
            status: "idle".to_string(),
        }
    }
}

/// Baseline reference agent (canonical systems engineering designation for smart artifacts supervised by a specialist)
pub type BaselineReferenceAgent = RelicAgent;

/// Relic type - smart artifact supervised by a specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicAgent {
    pub id: String,
    pub name: String,          // "DisplayBuffer", "KnowledgeStore", "OrchestratorCore", etc.
    pub supervisor_id: String, // ID of supervising specialist
    pub role: String,          // "Visual Operator", "Prophetic Synthesist", etc.
    pub persona: String,       // Personality flavor text
    pub cognitive_bias: CognitiveBias,
    pub hox_preset_path: String,          // Path to hox_relic_<name>.json
    pub enzyme_subset: Vec<String>,       // Allowlisted enzymes for this relic
    pub interval_ms: u64,                 // Polling interval in milliseconds
    pub metadata: HashMap<String, Value>, // Extensible metadata
    pub model_path: String,               // Path to GGUF model file
    pub model_hash: String,               // SHA256 hash of model file
    pub status: String,                   // Current operational status
}

impl Agent for RelicAgent {
    fn agent_id(&self) -> &str {
        &self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Relic
    }

    fn get_persona(&self) -> &str {
        &self.persona
    }

    fn get_cognitive_bias(&self) -> &CognitiveBias {
        &self.cognitive_bias
    }

    fn get_role(&self) -> &str {
        &self.role
    }
}

impl Default for RelicAgent {
    fn default() -> Self {
        let paths = WorkspacePaths::discover();
        RelicAgent {
            id: "relic_default".to_string(),
            name: "Template".to_string(),
            supervisor_id: "specialist_template".to_string(),
            role: "Observer".to_string(),
            persona: "A silent artifact observing systems.".to_string(),
            cognitive_bias: CognitiveBias::default(),
            hox_preset_path: paths
                .registry()
                .join("hox_relic_template.json")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec![],
            interval_ms: 30000,
            metadata: HashMap::new(),
            model_path: String::new(),
            model_hash: String::new(),
            status: "idle".to_string(),
        }
    }
}

/// User class - human end-user interacting with the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAgent {
    pub id: String,
    pub username: String,
    pub role: String, // "Administrator", "Operator", "Observer"
    pub persona: String,
    pub cognitive_bias: CognitiveBias,
    pub permissions: Vec<String>,       // List of allowed operations
    pub active_session: Option<String>, // Current session ID if active
}

impl Agent for UserAgent {
    fn agent_id(&self) -> &str {
        &self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::User
    }

    fn get_persona(&self) -> &str {
        &self.persona
    }

    fn get_cognitive_bias(&self) -> &CognitiveBias {
        &self.cognitive_bias
    }

    fn get_role(&self) -> &str {
        &self.role
    }
}

impl UserAgent {
    pub fn new(id: impl Into<String>, username: impl Into<String>, role: impl Into<String>, persona: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            username: username.into(),
            role: role.into(),
            persona: persona.into(),
            cognitive_bias: CognitiveBias::default(),
            permissions: vec!["all".to_string()],
            active_session: None,
        }
    }
}

impl Default for UserAgent {
    fn default() -> Self {
        UserAgent {
            id: "user_default".to_string(),
            username: "anonymous".to_string(),
            role: "Observer".to_string(),
            persona: "A curious human participant in the hive.".to_string(),
            cognitive_bias: CognitiveBias::default(),
            permissions: vec!["read".to_string(), "observe".to_string()],
            active_session: None,
        }
    }
}

/// Base Agent (Aaroneous/Agent-Zero)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseAgent {
    pub id: String,
    pub name: String,        // "Aaroneous"
    pub designation: String, // "Agent-Zero"
    pub persona: String,
    pub cognitive_bias: CognitiveBias,
    pub active_specialists: Vec<String>, // IDs of active specialist instances
    pub active_relics: Vec<String>,      // IDs of active relic instances
    pub active_users: Vec<String>,       // IDs of active user sessions
}

impl Agent for BaseAgent {
    fn agent_id(&self) -> &str {
        &self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::BaseAgent
    }

    fn get_persona(&self) -> &str {
        &self.persona
    }

    fn get_cognitive_bias(&self) -> &CognitiveBias {
        &self.cognitive_bias
    }

    fn get_role(&self) -> &str {
        "Orchestrator"
    }
}

impl Default for BaseAgent {
    fn default() -> Self {
        BaseAgent {
            id: "aaroneous_agent_zero".to_string(),
            name: "Aaroneous".to_string(),
            designation: "Agent-Zero".to_string(),
            persona: "The foundational orchestrator of the synthetic intelligence hive. Stable, reliable, and recursive.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 85,
                creative_variance: 40,
                audit_strictness: 90,
            },
            active_specialists: Vec::new(),
            active_relics: Vec::new(),
            active_users: Vec::new(),
        }
    }
}

/// Predefined specialist factory methods
pub fn create_specialist(name: &str) -> Option<SpecialistAgent> {
    let paths = crate::workspace::WorkspacePaths::discover();
    match name.to_lowercase().as_str() {
        "presenter" => Some(SpecialistAgent {
            id: "specialist_presenter".to_string(),
            name: "Presenter".to_string(),
            domain: Domain::UserInterface,
            role: "UI/UX Designer & Experience Architect".to_string(),
            persona:
                "Creative, intuitive, and empathetic. Visualizes the user experience holistically."
                    .to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 65,
                creative_variance: 95,
                audit_strictness: 40,
            },
            supervised_relic: Some("relic_display_buffer".to_string()),
            hox_preset_path: paths
                .sovereign_hox_preset("presenter")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec!["sensor_node".to_string(), "tensor_forge".to_string()],
            interval_ms: 20000,
            model_path: paths.sovereign_model("presenter").to_string_lossy().to_string(),
            model_hash: "70fe5af18c8f804a2e071fed22f72327f7beb59acdb905a476acbd40cb5513ee"
                .to_string(),
            status: "active".to_string(),
        }),
        "synthesizer" => Some(SpecialistAgent {
            id: "specialist_synthesizer".to_string(),
            name: "Synthesizer".to_string(),
            domain: Domain::Knowledge,
            role: "Knowledge Synthesist & Architect".to_string(),
            persona: "Prophetic and visionary. Sees patterns across vast knowledge domains."
                .to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 90,
                creative_variance: 75,
                audit_strictness: 60,
            },
            supervised_relic: Some("relic_knowledge_store".to_string()),
            hox_preset_path: paths
                .sovereign_hox_preset("synthesizer")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec!["thought_kernel".to_string(), "tensor_forge".to_string()],
            interval_ms: 25000,
            model_path: paths
                .sovereign_model("synthesizer")
                .to_string_lossy()
                .to_string(),
            model_hash: "b75872377e2b5fe391b4c168d1184c6be24ea362de63f788875c6493eccb55e2"
                .to_string(),
            status: "active".to_string(),
        }),
        "orchestrator" => Some(SpecialistAgent {
            id: "specialist_orchestrator".to_string(),
            name: "Orchestrator".to_string(),
            domain: Domain::Leadership,
            role: "Strategic Orchestrator & Leader".to_string(),
            persona: "All-seeing and cunning. Orchestrates with precision and foresight."
                .to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 88,
                creative_variance: 55,
                audit_strictness: 75,
            },
            supervised_relic: Some("relic_orchestrator_core".to_string()),
            hox_preset_path: paths
                .sovereign_hox_preset("orchestrator")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec!["thought_kernel".to_string(), "nat_bridge".to_string()],
            interval_ms: 30000,
            model_path: paths.sovereign_model("orchestrator").to_string_lossy().to_string(),
            model_hash: "ec614427643249d67a927ad5ad5b19e71d56eb3f3ec4d63c58ff0a5f2b17033c"
                .to_string(),
            status: "active".to_string(),
        }),
        "archivist" => Some(SpecialistAgent {
            id: "specialist_archivist".to_string(),
            name: "Archivist".to_string(),
            domain: Domain::Experience,
            role: "Experience & Memory Curator".to_string(),
            persona:
                "Exploratory and sensory. Experiences and celebrates the richness of the hive."
                    .to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 70,
                creative_variance: 90,
                audit_strictness: 35,
            },
            supervised_relic: Some("relic_memory_index".to_string()),
            hox_preset_path: paths
                .sovereign_hox_preset("archivist")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec!["sensor_node".to_string(), "thought_kernel".to_string()],
            interval_ms: 35000,
            model_path: paths
                .sovereign_model("archivist")
                .to_string_lossy()
                .to_string(),
            model_hash: "4cdca60ca840f3de4f4a4b12649ac05136f2637971f729b1075639492834e3d2"
                .to_string(),
            status: "active".to_string(),
        }),
        "fabricator" => Some(SpecialistAgent {
            id: "specialist_fabricator".to_string(),
            name: "Fabricator".to_string(),
            domain: Domain::Manufacturing,
            role: "Manufacturing & Execution Engine".to_string(),
            persona: "Methodical and constructive. Builds and manifests with precision."
                .to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 80,
                creative_variance: 60,
                audit_strictness: 85,
            },
            supervised_relic: Some("relic_compiler_core".to_string()),
            hox_preset_path: paths
                .sovereign_hox_preset("fabricator")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec!["tensor_forge".to_string(), "thought_kernel".to_string()],
            interval_ms: 22000,
            model_path: paths
                .sovereign_model("fabricator")
                .to_string_lossy()
                .to_string(),
            model_hash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
            status: "active".to_string(),
        }),
        "sentinel" => Some(SpecialistAgent {
            id: "specialist_sentinel".to_string(),
            name: "Sentinel".to_string(),
            domain: Domain::Security,
            role: "Security Warden & Sentinel".to_string(),
            persona: "Vigilant and scrutinizing. Questions everything, trusts nothing.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 95,
                creative_variance: 15,
                audit_strictness: 100,
            },
            supervised_relic: Some("relic_audit_engine".to_string()),
            hox_preset_path: paths
                .sovereign_hox_preset("sentinel")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec!["nat_bridge".to_string(), "sensor_node".to_string()],
            interval_ms: 15000,
            model_path: paths.sovereign_model("sentinel").to_string_lossy().to_string(),
            model_hash: "4cdca60ca840f3de4f4a4b12649ac05136f2637971f729b1075639492834e3d2"
                .to_string(),
            status: "active".to_string(),
        }),
        _ => None,
    }
}

/// Predefined reference artifact factory methods (canonical systems designation)
pub fn create_reference_agent(name: &str, supervisor_id: &str) -> Option<BaselineReferenceAgent> {
    create_relic(name, supervisor_id)
}

/// Predefined relic factory methods
pub fn create_relic(name: &str, supervisor_id: &str) -> Option<RelicAgent> {
    let paths = crate::workspace::WorkspacePaths::discover();
    match name.to_lowercase().as_str() {
        "display_buffer" => Some(RelicAgent {
            id: "relic_display_buffer".to_string(),
            name: "DisplayBuffer".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Visual Operator & Perception Engine".to_string(),
            persona: "Transparent and layered. Sees and translates visual and spatial information."
                .to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 70,
                creative_variance: 80,
                audit_strictness: 45,
            },
            hox_preset_path: paths
                .relic_hox_preset("display_buffer")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec!["sensor_node".to_string(), "tensor_forge".to_string()],
            interval_ms: 18000,
            metadata: HashMap::new(),
            model_path: paths.sovereign_model("display_buffer").to_string_lossy().to_string(),
            model_hash: "70fe5af18c8f804a2e071fed22f72327f7beb59acdb905a476acbd40cb5513ee"
                .to_string(),
            status: "active".to_string(),
        }),
        "knowledge_store" => Some(RelicAgent {
            id: "relic_knowledge_store".to_string(),
            name: "KnowledgeStore".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Prophetic Knowledge Index".to_string(),
            persona: "Ancient and mystical. Holds and reveals hidden patterns of knowledge."
                .to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 92,
                creative_variance: 70,
                audit_strictness: 55,
            },
            hox_preset_path: paths
                .relic_hox_preset("knowledge_store")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec!["thought_kernel".to_string(), "tensor_forge".to_string()],
            interval_ms: 28000,
            metadata: HashMap::new(),
            model_path: paths
                .sovereign_model("knowledge_store")
                .to_string_lossy()
                .to_string(),
            model_hash: "b75872377e2b5fe391b4c168d1184c6be24ea362de63f788875c6493eccb55e2"
                .to_string(),
            status: "active".to_string(),
        }),
        "orchestrator_core" => Some(RelicAgent {
            id: "relic_orchestrator_core".to_string(),
            name: "OrchestratorCore".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Resource Allocator & Coordinator".to_string(),
            persona: "Sovereign and precise. Allocates resources with strategic foresight."
                .to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 88,
                creative_variance: 50,
                audit_strictness: 80,
            },
            hox_preset_path: paths
                .relic_hox_preset("orchestrator_core")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec!["thought_kernel".to_string(), "nat_bridge".to_string()],
            interval_ms: 32000,
            metadata: HashMap::new(),
            model_path: paths
                .sovereign_model("orchestrator_core")
                .to_string_lossy()
                .to_string(),
            model_hash: "ec614427643249d67a927ad5ad5b19e71d56eb3f3ec4d63c58ff0a5f2b17033c"
                .to_string(),
            status: "active".to_string(),
        }),
        "memory_index" => Some(RelicAgent {
            id: "relic_memory_index".to_string(),
            name: "MemoryIndex".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Experiential Memory Librarian".to_string(),
            persona: "Omniscient and revelatory. Holds collective memories and patterns."
                .to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 85,
                creative_variance: 85,
                audit_strictness: 50,
            },
            hox_preset_path: paths.relic_hox_preset("memory_index").to_string_lossy().to_string(),
            enzyme_subset: vec!["sensor_node".to_string(), "thought_kernel".to_string()],
            interval_ms: 40000,
            metadata: HashMap::new(),
            model_path: paths.sovereign_model("memory_index").to_string_lossy().to_string(),
            model_hash: "4cdca60ca840f3de4f4a4b12649ac05136f2637971f729b1075639492834e3d2"
                .to_string(),
            status: "active".to_string(),
        }),
        "compiler_core" => Some(RelicAgent {
            id: "relic_compiler_core".to_string(),
            name: "CompilerCore".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Manufacturing & Synthesis Executor".to_string(),
            persona: "Tireless and exacting. Forges possibilities into reality.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 82,
                creative_variance: 55,
                audit_strictness: 88,
            },
            hox_preset_path: paths
                .relic_hox_preset("compiler_core")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec!["tensor_forge".to_string(), "thought_kernel".to_string()],
            interval_ms: 20000,
            metadata: HashMap::new(),
            model_path: paths.sovereign_model("compiler_core").to_string_lossy().to_string(),
            model_hash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
            status: "active".to_string(),
        }),
        "audit_engine" => Some(RelicAgent {
            id: "relic_audit_engine".to_string(),
            name: "AuditEngine".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Security Monitor & Threat Detector".to_string(),
            persona: "Watchful and paranoid. Never sleeps, always vigilant.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 96,
                creative_variance: 10,
                audit_strictness: 100,
            },
            hox_preset_path: paths
                .relic_hox_preset("audit_engine")
                .to_string_lossy()
                .to_string(),
            enzyme_subset: vec!["nat_bridge".to_string(), "sensor_node".to_string()],
            interval_ms: 12000,
            metadata: HashMap::new(),
            model_path: paths
                .sovereign_model("audit_engine")
                .to_string_lossy()
                .to_string(),
            model_hash: "4cdca60ca840f3de4f4a4b12649ac05136f2637971f729b1075639492834e3d2"
                .to_string(),
            status: "active".to_string(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialist_creation_and_agent_trait() {
        let specialist = create_specialist("orchestrator").expect("orchestrator specialist should exist");
        assert_eq!(specialist.agent_id(), "specialist_orchestrator");
        assert_eq!(specialist.agent_type(), AgentType::Specialist);
        assert_eq!(specialist.domain, Domain::Leadership);
        assert_eq!(specialist.get_role(), "Strategic Orchestrator & Leader");
        assert!(specialist.get_cognitive_bias().analytical_depth >= 85);
        assert!(!specialist.get_persona().is_empty());
    }

    #[test]
    fn test_relic_creation_and_supervision() {
        let relic = create_relic("display_buffer", "specialist_presenter").expect("display_buffer relic should exist");
        assert_eq!(relic.agent_id(), "relic_display_buffer");
        assert_eq!(relic.agent_type(), AgentType::Relic);
        assert_eq!(relic.supervisor_id, "specialist_presenter");
        assert_eq!(relic.status, "active");
        assert_eq!(relic.get_role(), "Visual Operator & Perception Engine");
    }

    #[test]
    fn test_user_agent_instantiation() {
        let user = UserAgent::new("user_aaron", "Aaron", "Architect", "Curious and rigorous");
        assert_eq!(user.agent_id(), "user_aaron");
        assert_eq!(user.agent_type(), AgentType::User);
        assert_eq!(user.get_role(), "Architect");
        assert_eq!(user.get_persona(), "Curious and rigorous");
    }

    #[test]
    fn test_all_specialists_and_relics_catalogs() {
        let names = ["presenter", "synthesizer", "orchestrator", "sentinel", "archivist", "fabricator"];
        for name in &names {
            let s = create_specialist(name);
            assert!(s.is_some(), "Specialist {} should be constructable", name);
        }

        let relics = ["display_buffer", "knowledge_store", "orchestrator_core", "memory_index", "compiler_core", "audit_engine"];
        for r in &relics {
            let rel = create_relic(r, "sup_01");
            assert!(rel.is_some(), "Relic {} should be constructable", r);
        }
    }
}
