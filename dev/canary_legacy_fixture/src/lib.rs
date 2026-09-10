// Legacy module with deliberate violations for normalization testing

use serde::{Deserialize, Serialize};

/// Unaligned struct - missing #[repr(C)] and proper padding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyData {
    pub id: u32,
    pub value: f64,
    pub metadata: String,
}

impl LegacyData {
    /// Contains .unwrap() violation
    pub fn process(&self) -> Result<String, Box<dyn std::error::Error>> {
        let x: Option<i32> = None;
        let processed = x.unwrap(); // UNWRAP VIOLATION
        Ok(format!("Processed: {} {}", self.id, processed))
    }

    /// Contains panic! violation  
    pub fn unsafe_operation(&self) -> i32 {
        panic!("Operation failed"); // PANIC VIOLATION
    }

    /// Contains bare unsafe block
    pub fn raw_memory_access(&mut self) {
        unsafe {
            // Simulated raw memory operation
            println!("Raw access to legacy data");
        } // UNSAFE BLOCK VIOLATION
    }
}

/// Missing workspace inheritance - standalone module without proper Cargo.toml reference
#[derive(Debug)]
pub struct LegacyConfig {
    pub version: u32,
    pub enabled: bool,
}

impl Default for LegacyConfig {
    fn default() -> Self {
        Self {
            version: 1,
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_processing() {
        let data = LegacyData {
            id: 42,
            value: 3.14,
            metadata: "test".to_string(),
        };
        
        let result = data.process(); // This will panic in runtime due to unwrap
        assert!(result.is_err());
    }
}
