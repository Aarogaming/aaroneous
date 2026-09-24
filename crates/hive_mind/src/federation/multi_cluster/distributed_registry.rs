/// Distributed Specialist Registry: Cross-Hive Specialist Discovery
///
/// Maintains a distributed registry of specialists across all hives
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Remote specialist reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSpecialist {
    pub specialist_id: crate::federation::specialist::SpecialistId,
    pub hive_node_id: String,
    pub address: String,
    pub model_size_mb: u32,
    pub available: bool,
    pub capability_tags: Vec<String>,
    pub last_seen_ms: u64,
}

impl RemoteSpecialist {
    pub fn new(
        specialist_id: crate::federation::specialist::SpecialistId,
        hive_node_id: String,
        address: String,
    ) -> Self {
        Self {
            specialist_id,
            hive_node_id,
            address,
            model_size_mb: specialist_id.model_size_mb(),
            available: true,
            capability_tags: Vec::new(),
            last_seen_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

/// Distributed specialist registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedSpecialistRegistry {
    /// Map of (specialist_id, hive_node_id) -> RemoteSpecialist
    pub specialists:
        HashMap<(crate::federation::specialist::SpecialistId, String), RemoteSpecialist>,
    pub total_specialists: u32,
    pub last_sync_ms: u64,
}

impl DistributedSpecialistRegistry {
    pub fn new() -> Self {
        Self {
            specialists: HashMap::new(),
            total_specialists: 0,
            last_sync_ms: 0,
        }
    }

    /// Register a hive's specialists
    pub fn register_node(
        &mut self,
        node: crate::federation::multi_hive::HiveNode,
    ) -> Result<(), String> {
        for i in 0..6 {
            let specialist_id = match i {
                0 => crate::federation::specialist::SpecialistId::Sentinel,
                1 => crate::federation::specialist::SpecialistId::Visionary,
                2 => crate::federation::specialist::SpecialistId::Omnipresent,
                3 => crate::federation::specialist::SpecialistId::Symbiotic,
                4 => crate::federation::specialist::SpecialistId::Phygital,
                _ => crate::federation::specialist::SpecialistId::Archivist,
            };

            let remote =
                RemoteSpecialist::new(specialist_id, node.node_id.clone(), node.address.clone());

            self.specialists
                .insert((specialist_id, node.node_id.clone()), remote);
            self.total_specialists += 1;
        }

        self.last_sync_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(())
    }

    /// Find all instances of a specialist
    pub fn find_specialist(
        &self,
        specialist_id: crate::federation::specialist::SpecialistId,
    ) -> Vec<RemoteSpecialist> {
        self.specialists
            .values()
            .filter(|s| s.specialist_id == specialist_id)
            .cloned()
            .collect()
    }

    /// Find specialists in a specific hive
    pub fn find_hive_specialists(&self, hive_node_id: &str) -> Vec<RemoteSpecialist> {
        self.specialists
            .values()
            .filter(|s| s.hive_node_id == hive_node_id)
            .cloned()
            .collect()
    }

    /// Find specialist with specific capability
    pub fn find_by_capability(&self, capability: &str) -> Vec<RemoteSpecialist> {
        self.specialists
            .values()
            .filter(|s| s.capability_tags.contains(&capability.to_string()))
            .cloned()
            .collect()
    }

    /// Update specialist availability
    pub fn update_availability(
        &mut self,
        specialist_id: crate::federation::specialist::SpecialistId,
        hive_node_id: &str,
        available: bool,
    ) {
        let key = (specialist_id, hive_node_id.to_string());
        if let Some(specialist) = self.specialists.get_mut(&key) {
            specialist.available = available;
            specialist.last_seen_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
        }
    }

    /// Get registry statistics
    pub fn stats(&self) -> RegistryStats {
        let available = self.specialists.values().filter(|s| s.available).count();
        let unavailable = self.specialists.len() - available;

        RegistryStats {
            total_entries: self.specialists.len(),
            available_specialists: available,
            unavailable_specialists: unavailable,
            total_model_size_mb: self.specialists.values().map(|s| s.model_size_mb).sum(),
        }
    }
}

impl Default for DistributedSpecialistRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    pub total_entries: usize,
    pub available_specialists: usize,
    pub unavailable_specialists: usize,
    pub total_model_size_mb: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_specialist_creation() {
        let specialist = RemoteSpecialist::new(
            crate::federation::specialist::SpecialistId::Visionary,
            "hive-1".to_string(),
            "127.0.0.1:8001".to_string(),
        );
        assert_eq!(
            specialist.specialist_id,
            crate::federation::specialist::SpecialistId::Visionary
        );
        assert!(specialist.available);
    }

    #[test]
    fn test_registry_register_node() {
        let mut registry = DistributedSpecialistRegistry::new();
        let node = crate::federation::multi_hive::HiveNode::new(
            "hive-1".to_string(),
            "127.0.0.1:8001".to_string(),
        );

        let result = registry.register_node(node);
        assert!(result.is_ok());
        assert_eq!(registry.total_specialists, 6);
    }

    #[test]
    fn test_registry_find_specialist() {
        let mut registry = DistributedSpecialistRegistry::new();
        let node = crate::federation::multi_hive::HiveNode::new(
            "hive-1".to_string(),
            "127.0.0.1:8001".to_string(),
        );
        registry.register_node(node).ok();

        let specialists =
            registry.find_specialist(crate::federation::specialist::SpecialistId::Visionary);
        assert_eq!(specialists.len(), 1);
    }

    #[test]
    fn test_registry_stats() {
        let mut registry = DistributedSpecialistRegistry::new();
        let node = crate::federation::multi_hive::HiveNode::new(
            "hive-1".to_string(),
            "127.0.0.1:8001".to_string(),
        );
        registry.register_node(node).ok();

        let stats = registry.stats();
        assert_eq!(stats.total_entries, 6);
        assert_eq!(stats.available_specialists, 6);
    }
}
