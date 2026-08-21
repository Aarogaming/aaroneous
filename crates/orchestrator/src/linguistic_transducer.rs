//! Linguistic Transducer module for bridging Merlin's CAS calculations to natural language text.

use std::collections::HashMap;

/// The main Linguistic Transducer struct
pub struct LinguisticTransducer {
    /// Mapping of CAS values to natural language representations
    cas_to_text: HashMap<String, String>,
    /// Mapping of natural language to CAS values
    text_to_cas: HashMap<String, String>,
}

impl Default for LinguisticTransducer {
    fn default() -> Self {
        Self::new()
    }
}

impl LinguisticTransducer {
    /// Creates a new instance of the Linguistic Transducer
    pub fn new() -> Self {
        Self {
            cas_to_text: HashMap::new(),
            text_to_cas: HashMap::new(),
        }
    }

    /// Translates a CAS value to natural language text
    pub fn cas_to_text(&self, cas_value: &str) -> Option<&str> {
        self.cas_to_text.get(cas_value).map(|s| s.as_str())
    }

    /// Translates natural language text to a CAS value
    pub fn text_to_cas(&self, text: &str) -> Option<&str> {
        self.text_to_cas.get(text).map(|s| s.as_str())
    }

    /// Adds a mapping from CAS value to natural language text
    pub fn add_cas_mapping(&mut self, cas_value: String, text: String) {
        self.cas_to_text.insert(cas_value.clone(), text.clone());
        self.text_to_cas.insert(text, cas_value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_transducer() {
        let transducer = LinguisticTransducer::new();
        assert!(transducer.cas_to_text.is_empty());
        assert!(transducer.text_to_cas.is_empty());
    }

    #[test]
    fn test_add_mapping() {
        let mut transducer = LinguisticTransducer::new();
        transducer.add_cas_mapping("cas_123".to_string(), "hello world".to_string());

        assert_eq!(transducer.cas_to_text("cas_123"), Some("hello world"));
        assert_eq!(transducer.text_to_cas("hello world"), Some("cas_123"));
    }
}
