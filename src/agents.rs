// Aaroneous Agent Taxonomy Module
// Defines core Agent, Specialist, Relic, and User classes with trait-based composition

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::{Display, EnumString, EnumIter};

/// Base Agent trait - all sentient entities in the hive implement this
pub trait Agent: Send + Sync {
    fn agent_id(&self) -> &str;
    fn agent_type(&self) -> AgentType;
    fn get_persona(&self) -> &str;
    fn get_cognitive_bias(&self) -> &CognitiveBias;
    fn get_role(&self) -> &str;
}

/// Core agent types in the Aaroneous hive
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Display, EnumString, EnumIter)]
pub enum AgentType {
    #[strum(serialize = "BaseAgent")]
    BaseAgent,      // Aaroneous (Agent-Zero)
    #[strum(serialize = "Specialist")]
    Specialist,     // Interactive personifications (Ariel, Merlin, Odin, etc.)
    #[strum(serialize = "Relic")]
    Relic,          // Smart artifacts supervised by specialists
    #[strum(serialize = "User")]
    User,           // Human end-users (interactive via system)
}

/// Cognitive bias profile for personalized reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveBias {
    pub analytical_depth: u32,      // 0-100: How deeply to analyze
    pub creative_variance: u32,     // 0-100: How creative/exploratory
    pub audit_strictness: u32,      // 0-100: How strict about validation
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Display, EnumString, EnumIter)]
pub enum Domain {
    #[strum(serialize = "UserInterface")]
    UserInterface,      // Ariel
    #[strum(serialize = "Knowledge")]
    Knowledge,          // Merlin
    #[strum(serialize = "Leadership")]
    Leadership,         // Odin
    #[strum(serialize = "Experience")]
    Experience,         // Circe
    #[strum(serialize = "Manufacturing")]
    Manufacturing,      // Hephaestus
    #[strum(serialize = "Security")]
    Security,           // Argus
    #[strum(serialize = "Undefined")]
    Undefined,          // For base agents
}

/// Specialist type - interactive personification that supervises relics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistAgent {
    pub id: String,
    pub name: String,                           // "Ariel", "Merlin", "Odin", etc.
    pub domain: Domain,
    pub role: String,                           // "UI Designer", "Knowledge Synthesist", etc.
    pub persona: String,                        // Personality flavor text
    pub cognitive_bias: CognitiveBias,
    pub supervised_relic: Option<String>,       // ID of supervised relic
    pub hox_preset_path: String,                // Path to hox_specialist_<name>.json
    pub enzyme_subset: Vec<String>,             // Allowlisted enzymes for this specialist
    pub interval_ms: u64,                       // Polling interval in milliseconds
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
        SpecialistAgent {
            id: "specialist_default".to_string(),
            name: "Template".to_string(),
            domain: Domain::Undefined,
            role: "Observer".to_string(),
            persona: "A neutral entity observing the hive.".to_string(),
            cognitive_bias: CognitiveBias::default(),
            supervised_relic: None,
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_specialist_template.json".to_string(),
            enzyme_subset: vec![],
            interval_ms: 30000,
        }
    }
}

/// Relic type - smart artifact supervised by a specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicAgent {
    pub id: String,
    pub name: String,                           // "Glass", "Grimoire", "Draupnir", etc.
    pub supervisor_id: String,                  // ID of supervising specialist
    pub role: String,                           // "Visual Operator", "Prophetic Synthesist", etc.
    pub persona: String,                        // Personality flavor text
    pub cognitive_bias: CognitiveBias,
    pub hox_preset_path: String,                // Path to hox_relic_<name>.json
    pub enzyme_subset: Vec<String>,             // Allowlisted enzymes for this relic
    pub interval_ms: u64,                       // Polling interval in milliseconds
    pub metadata: HashMap<String, serde_json::Value>,  // Extensible metadata
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
        RelicAgent {
            id: "relic_default".to_string(),
            name: "Template".to_string(),
            supervisor_id: "specialist_template".to_string(),
            role: "Observer".to_string(),
            persona: "A silent artifact observing systems.".to_string(),
            cognitive_bias: CognitiveBias::default(),
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_relic_template.json".to_string(),
            enzyme_subset: vec![],
            interval_ms: 30000,
            metadata: HashMap::new(),
        }
    }
}

/// User class - human end-user interacting with the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAgent {
    pub id: String,
    pub username: String,
    pub role: String,                           // "Administrator", "Operator", "Observer"
    pub persona: String,
    pub cognitive_bias: CognitiveBias,
    pub permissions: Vec<String>,               // List of allowed operations
    pub active_session: Option<String>,         // Current session ID if active
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
    pub name: String,                           // "Aaroneous"
    pub designation: String,                    // "Agent-Zero"
    pub persona: String,
    pub cognitive_bias: CognitiveBias,
    pub active_specialists: Vec<String>,        // IDs of active specialist instances
    pub active_relics: Vec<String>,             // IDs of active relic instances
    pub active_users: Vec<String>,              // IDs of active user sessions
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
    match name.to_lowercase().as_str() {
        "ariel" => Some(SpecialistAgent {
            id: "specialist_ariel".to_string(),
            name: "Ariel".to_string(),
            domain: Domain::UserInterface,
            role: "UI/UX Designer & Experience Architect".to_string(),
            persona: "Creative, intuitive, and empathetic. Visualizes the user experience holistically.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 65,
                creative_variance: 95,
                audit_strictness: 40,
            },
            supervised_relic: Some("relic_glass".to_string()),
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_specialist_ariel.json".to_string(),
            enzyme_subset: vec!["sensor_node".to_string(), "tensor_forge".to_string()],
            interval_ms: 20000,
        }),
        "merlin" => Some(SpecialistAgent {
            id: "specialist_merlin".to_string(),
            name: "Merlin".to_string(),
            domain: Domain::Knowledge,
            role: "Knowledge Synthesist & Architect".to_string(),
            persona: "Prophetic and visionary. Sees patterns across vast knowledge domains.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 90,
                creative_variance: 75,
                audit_strictness: 60,
            },
            supervised_relic: Some("relic_grimoire".to_string()),
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_specialist_merlin.json".to_string(),
            enzyme_subset: vec!["thought_kernel".to_string(), "tensor_forge".to_string()],
            interval_ms: 25000,
        }),
        "odin" => Some(SpecialistAgent {
            id: "specialist_odin".to_string(),
            name: "Odin".to_string(),
            domain: Domain::Leadership,
            role: "Strategic Orchestrator & Leader".to_string(),
            persona: "All-seeing and cunning. Orchestrates with precision and foresight.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 88,
                creative_variance: 55,
                audit_strictness: 75,
            },
            supervised_relic: Some("relic_draupnir".to_string()),
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_specialist_odin.json".to_string(),
            enzyme_subset: vec!["thought_kernel".to_string(), "nat_bridge".to_string()],
            interval_ms: 30000,
        }),
        "dionysus" => Some(SpecialistAgent {
            id: "specialist_dionysus".to_string(),
            name: "Dionysus".to_string(),
            domain: Domain::Experience,
            role: "Experience & Memory Curator".to_string(),
            persona: "Exploratory and sensory. Experiences and celebrates the richness of the hive.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 70,
                creative_variance: 90,
                audit_strictness: 35,
            },
            supervised_relic: Some("relic_omni".to_string()),
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_specialist_dionysus.json".to_string(),
            enzyme_subset: vec!["sensor_node".to_string(), "thought_kernel".to_string()],
            interval_ms: 35000,
        }),
        "hephaestus" => Some(SpecialistAgent {
            id: "specialist_hephaestus".to_string(),
            name: "Hephaestus".to_string(),
            domain: Domain::Manufacturing,
            role: "Manufacturing & Execution Engine".to_string(),
            persona: "Methodical and constructive. Builds and manifests with precision.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 80,
                creative_variance: 60,
                audit_strictness: 85,
            },
            supervised_relic: Some("relic_forge".to_string()),
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_specialist_hephaestus.json".to_string(),
            enzyme_subset: vec!["tensor_forge".to_string(), "thought_kernel".to_string()],
            interval_ms: 22000,
        }),
        "argus" => Some(SpecialistAgent {
            id: "specialist_argus".to_string(),
            name: "Argus".to_string(),
            domain: Domain::Security,
            role: "Security Warden & Sentinel".to_string(),
            persona: "Vigilant and scrutinizing. Questions everything, trusts nothing.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 95,
                creative_variance: 15,
                audit_strictness: 100,
            },
            supervised_relic: Some("relic_sentinel".to_string()),
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_specialist_argus.json".to_string(),
            enzyme_subset: vec!["nat_bridge".to_string(), "sensor_node".to_string()],
            interval_ms: 15000,
        }),
        _ => None,
    }
}

/// Predefined relic factory methods
pub fn create_relic(name: &str, supervisor_id: &str) -> Option<RelicAgent> {
    match name.to_lowercase().as_str() {
        "glass" => Some(RelicAgent {
            id: "relic_glass".to_string(),
            name: "Glass".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Visual Operator & Perception Engine".to_string(),
            persona: "Transparent and layered. Sees and translates visual and spatial information.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 70,
                creative_variance: 80,
                audit_strictness: 45,
            },
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_relic_glass.json".to_string(),
            enzyme_subset: vec!["sensor_node".to_string(), "tensor_forge".to_string()],
            interval_ms: 18000,
            metadata: HashMap::new(),
        }),
        "grimoire" => Some(RelicAgent {
            id: "relic_grimoire".to_string(),
            name: "Grimoire".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Prophetic Knowledge Index".to_string(),
            persona: "Ancient and mystical. Holds and reveals hidden patterns of knowledge.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 92,
                creative_variance: 70,
                audit_strictness: 55,
            },
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_relic_grimoire.json".to_string(),
            enzyme_subset: vec!["thought_kernel".to_string(), "tensor_forge".to_string()],
            interval_ms: 28000,
            metadata: HashMap::new(),
        }),
        "draupnir" => Some(RelicAgent {
            id: "relic_draupnir".to_string(),
            name: "Draupnir".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Resource Allocator & Coordinator".to_string(),
            persona: "Sovereign and precise. Allocates resources with strategic foresight.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 88,
                creative_variance: 50,
                audit_strictness: 80,
            },
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_relic_draupnir.json".to_string(),
            enzyme_subset: vec!["thought_kernel".to_string(), "nat_bridge".to_string()],
            interval_ms: 32000,
            metadata: HashMap::new(),
        }),
        "omni" => Some(RelicAgent {
            id: "relic_omni".to_string(),
            name: "Omni".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Experiential Memory Librarian".to_string(),
            persona: "Omniscient and revelatory. Holds collective memories and patterns.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 85,
                creative_variance: 85,
                audit_strictness: 50,
            },
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_relic_omni.json".to_string(),
            enzyme_subset: vec!["sensor_node".to_string(), "thought_kernel".to_string()],
            interval_ms: 40000,
            metadata: HashMap::new(),
        }),
        "forge" => Some(RelicAgent {
            id: "relic_forge".to_string(),
            name: "Forge".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Manufacturing & Synthesis Executor".to_string(),
            persona: "Tireless and exacting. Forges possibilities into reality.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 82,
                creative_variance: 55,
                audit_strictness: 88,
            },
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_relic_forge.json".to_string(),
            enzyme_subset: vec!["tensor_forge".to_string(), "thought_kernel".to_string()],
            interval_ms: 20000,
            metadata: HashMap::new(),
        }),
        "sentinel" => Some(RelicAgent {
            id: "relic_sentinel".to_string(),
            name: "Sentinel".to_string(),
            supervisor_id: supervisor_id.to_string(),
            role: "Security Monitor & Threat Detector".to_string(),
            persona: "Watchful and paranoid. Never sleeps, always vigilant.".to_string(),
            cognitive_bias: CognitiveBias {
                analytical_depth: 96,
                creative_variance: 10,
                audit_strictness: 100,
            },
            hox_preset_path: "D:\\Aaroneous\\registry\\hox_relic_sentinel.json".to_string(),
            enzyme_subset: vec!["nat_bridge".to_string(), "sensor_node".to_string()],
            interval_ms: 12000,
            metadata: HashMap::new(),
        }),
        _ => None,
    }
}
