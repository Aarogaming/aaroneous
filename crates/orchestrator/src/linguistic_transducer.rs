//! Linguistic Transducer module for bridging CAS (Control Abstraction Schema)
//! calculations to natural language text and specialist dispatch.
//!
//! CAS is the machine-native vocabulary that agents use internally.
//! This transducer converts between human-readable text and CAS opcodes,
//! enabling the agency model where users give intent in natural language
//! and specialists execute via CAS commands.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A CAS (Control Abstraction Schema) command
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CasCommand {
    pub opcode: u8,
    pub mnemonic: String,
    pub description: String,
    pub domain: String,
}

/// Pre-defined CAS vocabulary for the Aaroneous agency
pub fn default_cas_vocabulary() -> Vec<CasCommand> {
    vec![
        CasCommand { opcode: 0x01, mnemonic: "EXECUTE".to_string(), description: "Execute a task or command".to_string(), domain: "general".to_string() },
        CasCommand { opcode: 0x02, mnemonic: "ANALYZE".to_string(), description: "Analyze data or code".to_string(), domain: "knowledge".to_string() },
        CasCommand { opcode: 0x03, mnemonic: "GENERATE".to_string(), description: "Generate new code or content".to_string(), domain: "manufacturing".to_string() },
        CasCommand { opcode: 0x04, mnemonic: "REVIEW".to_string(), description: "Review and audit output".to_string(), domain: "security".to_string() },
        CasCommand { opcode: 0x05, mnemonic: "REFINE".to_string(), description: "Refactor or improve existing code".to_string(), domain: "manufacturing".to_string() },
        CasCommand { opcode: 0x06, mnemonic: "VISUALIZE".to_string(), description: "Create visual representation".to_string(), domain: "user_interface".to_string() },
        CasCommand { opcode: 0x07, mnemonic: "REMEMBER".to_string(), description: "Store knowledge or experience".to_string(), domain: "experience".to_string() },
        CasCommand { opcode: 0x08, mnemonic: "RECALL".to_string(), description: "Retrieve stored knowledge".to_string(), domain: "knowledge".to_string() },
        CasCommand { opcode: 0x09, mnemonic: "DEPLOY".to_string(), description: "Deploy or ship to production".to_string(), domain: "manufacturing".to_string() },
        CasCommand { opcode: 0x0A, mnemonic: "DEFEND".to_string(), description: "Security check or threat detection".to_string(), domain: "security".to_string() },
        CasCommand { opcode: 0x10, mnemonic: "PLAN".to_string(), description: "Strategic planning and orchestration".to_string(), domain: "leadership".to_string() },
        CasCommand { opcode: 0x11, mnemonic: "DELEGATE".to_string(), description: "Assign task to specialist".to_string(), domain: "leadership".to_string() },
        CasCommand { opcode: 0x12, mnemonic: "COORDINATE".to_string(), description: "Cross-specialist coordination".to_string(), domain: "leadership".to_string() },
        CasCommand { opcode: 0x20, mnemonic: "READ_FILE".to_string(), description: "Read file contents".to_string(), domain: "general".to_string() },
        CasCommand { opcode: 0x21, mnemonic: "WRITE_FILE".to_string(), description: "Write file contents".to_string(), domain: "general".to_string() },
        CasCommand { opcode: 0x22, mnemonic: "SEARCH".to_string(), description: "Search codebase or knowledge".to_string(), domain: "knowledge".to_string() },
        CasCommand { opcode: 0x23, mnemonic: "TEST".to_string(), description: "Run tests or validate".to_string(), domain: "security".to_string() },
        CasCommand { opcode: 0xFF, mnemonic: "NOP".to_string(), description: "No operation".to_string(), domain: "general".to_string() },
    ]
}

/// The main Linguistic Transducer struct
pub struct LinguisticTransducer {
    /// Mapping of CAS values to natural language representations
    cas_to_text: HashMap<String, String>,
    /// Mapping of natural language to CAS values
    text_to_cas: HashMap<String, String>,
    /// CAS opcode to command mapping
    opcode_to_command: HashMap<u8, CasCommand>,
    /// Mnemonic to opcode mapping
    mnemonic_to_opcode: HashMap<String, u8>,
}

impl Default for LinguisticTransducer {
    fn default() -> Self {
        Self::new()
    }
}

impl LinguisticTransducer {
    /// Creates a new instance of the Linguistic Transducer with default CAS vocabulary
    pub fn new() -> Self {
        let mut transducer = Self {
            cas_to_text: HashMap::new(),
            text_to_cas: HashMap::new(),
            opcode_to_command: HashMap::new(),
            mnemonic_to_opcode: HashMap::new(),
        };

        // Load default CAS vocabulary
        for cmd in default_cas_vocabulary() {
            transducer.register_command(cmd);
        }

        transducer
    }

    /// Register a CAS command in the transducer
    pub fn register_command(&mut self, cmd: CasCommand) {
        self.opcode_to_command.insert(cmd.opcode, cmd.clone());
        self.mnemonic_to_opcode
            .insert(cmd.mnemonic.clone(), cmd.opcode);
        self.cas_to_text
            .insert(cmd.opcode.to_string(), cmd.description.clone());
        self.text_to_cas
            .insert(cmd.description.to_lowercase(), cmd.opcode.to_string());
    }

    /// Translates a CAS opcode to its mnemonic
    pub fn opcode_to_mnemonic(&self, opcode: u8) -> Option<&str> {
        self.opcode_to_command
            .get(&opcode)
            .map(|cmd| cmd.mnemonic.as_str())
    }

    /// Translates a mnemonic to its opcode
    pub fn mnemonic_to_opcode(&self, mnemonic: &str) -> Option<u8> {
        self.mnemonic_to_opcode.get(mnemonic).copied()
    }

    /// Translates a CAS value to natural language text
    pub fn cas_to_text(&self, cas_value: &str) -> Option<&str> {
        self.cas_to_text.get(cas_value).map(|s| s.as_str())
    }

    /// Translates natural language text to a CAS value
    pub fn text_to_cas(&self, text: &str) -> Option<&str> {
        let lower = text.to_lowercase();
        self.text_to_cas.get(&lower).map(|s| s.as_str())
    }

    /// Adds a mapping from CAS value to natural language text
    pub fn add_cas_mapping(&mut self, cas_value: String, text: String) {
        self.cas_to_text.insert(cas_value.clone(), text.clone());
        self.text_to_cas.insert(text, cas_value);
    }

    /// Parse natural language intent into a CAS command
    pub fn parse_intent(&self, intent: &str) -> CasCommand {
        let lower = intent.to_lowercase();

        // Keyword matching for intent detection
        let mnemonic = if lower.contains("analyze") || lower.contains("examine") {
            "ANALYZE"
        } else if lower.contains("create") || lower.contains("generate") || lower.contains("build") {
            "GENERATE"
        } else if lower.contains("review") || lower.contains("audit") || lower.contains("check") {
            "REVIEW"
        } else if lower.contains("refactor") || lower.contains("improve") || lower.contains("optimize") {
            "REFINE"
        } else if lower.contains("visualize") || lower.contains("render") || lower.contains("display") {
            "VISUALIZE"
        } else if lower.contains("remember") || lower.contains("store") || lower.contains("save") {
            "REMEMBER"
        } else if lower.contains("recall") || lower.contains("retrieve") || lower.contains("find") {
            "RECALL"
        } else if lower.contains("deploy") || lower.contains("ship") || lower.contains("release") {
            "DEPLOY"
        } else if lower.contains("defend") || lower.contains("secure") || lower.contains("protect") {
            "DEFEND"
        } else if lower.contains("plan") || lower.contains("organize") || lower.contains("strategy") {
            "PLAN"
        } else if lower.contains("delegate") || lower.contains("assign") || lower.contains("route") {
            "DELEGATE"
        } else {
            "EXECUTE"
        };

        self.mnemonic_to_opcode
            .get(mnemonic)
            .and_then(|opcode| self.opcode_to_command.get(opcode))
            .cloned()
            .unwrap_or(CasCommand {
                opcode: 0xFF,
                mnemonic: "NOP".to_string(),
                description: "No operation".to_string(),
                domain: "general".to_string(),
            })
    }

    /// Map a CAS command to a specialist domain
    pub fn command_to_domain<'a>(&self, cmd: &'a CasCommand) -> &'a str {
        &cmd.domain
    }

    /// Get all registered commands
    pub fn vocabulary(&self) -> Vec<&CasCommand> {
        self.opcode_to_command.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_transducer() {
        let transducer = LinguisticTransducer::new();
        assert!(!transducer.cas_to_text.is_empty());
        assert!(!transducer.text_to_cas.is_empty());
        assert_eq!(transducer.opcode_to_command.len(), 18);
    }

    #[test]
    fn test_add_mapping() {
        let mut transducer = LinguisticTransducer::new();
        transducer.add_cas_mapping("cas_123".to_string(), "hello world".to_string());

        assert_eq!(transducer.cas_to_text("cas_123"), Some("hello world"));
        assert_eq!(transducer.text_to_cas("hello world"), Some("cas_123"));
    }

    #[test]
    fn test_opcode_to_mnemonic() {
        let transducer = LinguisticTransducer::new();
        assert_eq!(transducer.opcode_to_mnemonic(0x02), Some("ANALYZE"));
        assert_eq!(transducer.opcode_to_mnemonic(0x03), Some("GENERATE"));
        assert_eq!(transducer.opcode_to_mnemonic(0xFF), Some("NOP"));
    }

    #[test]
    fn test_mnemonic_to_opcode() {
        let transducer = LinguisticTransducer::new();
        assert_eq!(transducer.mnemonic_to_opcode("ANALYZE"), Some(0x02));
        assert_eq!(transducer.mnemonic_to_opcode("GENERATE"), Some(0x03));
        assert_eq!(transducer.mnemonic_to_opcode("NONEXISTENT"), None);
    }

    #[test]
    fn test_parse_intent_analyze() {
        let transducer = LinguisticTransducer::new();
        let cmd = transducer.parse_intent("Please analyze this code");
        assert_eq!(cmd.mnemonic, "ANALYZE");
        assert_eq!(cmd.domain, "knowledge");
    }

    #[test]
    fn test_parse_intent_generate() {
        let transducer = LinguisticTransducer::new();
        let cmd = transducer.parse_intent("Create a new function for sorting");
        assert_eq!(cmd.mnemonic, "GENERATE");
        assert_eq!(cmd.domain, "manufacturing");
    }

    #[test]
    fn test_parse_intent_review() {
        let transducer = LinguisticTransducer::new();
        let cmd = transducer.parse_intent("Review this pull request");
        assert_eq!(cmd.mnemonic, "REVIEW");
        assert_eq!(cmd.domain, "security");
    }

    #[test]
    fn test_parse_intent_refine() {
        let transducer = LinguisticTransducer::new();
        let cmd = transducer.parse_intent("Refactor the authentication module");
        assert_eq!(cmd.mnemonic, "REFINE");
    }

    #[test]
    fn test_command_to_domain() {
        let transducer = LinguisticTransducer::new();
        let cmd = transducer.parse_intent("analyze data");
        assert_eq!(transducer.command_to_domain(&cmd), "knowledge");
    }

    #[test]
    fn test_vocabulary() {
        let transducer = LinguisticTransducer::new();
        let vocab = transducer.vocabulary();
        assert!(vocab.len() >= 18);
    }
}
