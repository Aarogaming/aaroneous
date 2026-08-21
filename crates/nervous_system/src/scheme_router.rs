//! crates/nervous_system/src/scheme_router.rs
//! Universal Scheme URI Router & Capability-Based Access Control
//! Adapted from Redox OS Scheme Architecture (`libredox`).
//! Provides URI routing (`specialist://`, `synapse://`, `forge://`) and capability authorization.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Capability permissions bitflags (inspired by Redox OS capabilities)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityFlags(pub u32);

impl CapabilityFlags {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const FORGE: Self = Self(1 << 3);
    pub const ALL: Self = Self(0xFFFFFFFF);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

/// Parsed Scheme URI (e.g. `specialist://odin/task/consensus`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemeUri {
    pub scheme: String, // "specialist", "synapse", "forge", "memory"
    pub target: String, // "odin", "primary", "chimera", "merlin"
    pub path: String,   // "task/consensus", "slot/0x0400"
}

impl SchemeUri {
    pub fn parse(raw_uri: &str) -> Result<Self> {
        let parts: Vec<&str> = raw_uri.split("://").collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid Scheme URI format: {}", raw_uri));
        }

        let scheme = parts[0].to_lowercase();
        let rest = parts[1];

        let path_parts: Vec<&str> = rest.splitn(2, '/').collect();
        let target = path_parts[0].to_lowercase();
        let path = if path_parts.len() > 1 {
            path_parts[1].to_string()
        } else {
            String::new()
        };

        Ok(Self {
            scheme,
            target,
            path,
        })
    }

    pub fn to_string(&self) -> String {
        if self.path.is_empty() {
            format!("{}://{}", self.scheme, self.target)
        } else {
            format!("{}://{}/{}", self.scheme, self.target, self.path)
        }
    }
}

/// Capability Authorization Gate managing scheme access permissions
pub struct SchemeCapabilityGate {
    permissions: HashMap<String, CapabilityFlags>,
}

impl Default for SchemeCapabilityGate {
    fn default() -> Self {
        let mut gate = Self {
            permissions: HashMap::new(),
        };
        // Default permission rules
        gate.grant("specialist://odin", CapabilityFlags::ALL);
        gate.grant("specialist://hephaestus", CapabilityFlags(CapabilityFlags::READ.0 | CapabilityFlags::WRITE.0 | CapabilityFlags::FORGE.0));
        gate.grant("synapse://primary", CapabilityFlags(CapabilityFlags::READ.0 | CapabilityFlags::WRITE.0));
        gate
    }
}

impl SchemeCapabilityGate {
    pub fn grant(&mut self, uri_prefix: impl Into<String>, flags: CapabilityFlags) {
        self.permissions.insert(uri_prefix.into(), flags);
    }

    pub fn authorize(&self, uri: &SchemeUri, required_capability: CapabilityFlags) -> bool {
        let full_prefix = format!("{}://{}", uri.scheme, uri.target);
        if let Some(granted) = self.permissions.get(&full_prefix) {
            granted.contains(required_capability)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheme_uri_parsing() {
        let uri = SchemeUri::parse("specialist://odin/task/consensus").unwrap();
        assert_eq!(uri.scheme, "specialist");
        assert_eq!(uri.target, "odin");
        assert_eq!(uri.path, "task/consensus");
    }

    #[test]
    fn test_capability_authorization() {
        let gate = SchemeCapabilityGate::default();

        let odin_uri = SchemeUri::parse("specialist://odin/run").unwrap();
        assert!(gate.authorize(&odin_uri, CapabilityFlags::FORGE));

        let hephaestus_uri = SchemeUri::parse("specialist://hephaestus/forge").unwrap();
        assert!(gate.authorize(&hephaestus_uri, CapabilityFlags::FORGE));
        assert!(!gate.authorize(&hephaestus_uri, CapabilityFlags::EXECUTE)); // Not granted execute

        let unknown_uri = SchemeUri::parse("specialist://unknown/hack").unwrap();
        assert!(!gate.authorize(&unknown_uri, CapabilityFlags::READ));
    }
}
