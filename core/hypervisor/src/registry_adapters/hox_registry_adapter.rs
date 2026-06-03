use crate::registry::{SubRegistry, WorkspaceContext, EntityInfo, RegistryType, EntryHealth, PhaseEra};
use crate::hox_registry::HoxRegistry;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct HoxCapabilityRegistryAdapter {
    registry_type: RegistryType,
    inner_registry: Option<HoxRegistry>,
    is_initialized: bool,
}

impl HoxCapabilityRegistryAdapter {
    pub fn new() -> Self {
        Self {
            registry_type: RegistryType::HoxCapability,
            inner_registry: None,
            is_initialized: false,
        }
    }
}

impl SubRegistry for HoxCapabilityRegistryAdapter {
    fn initialize(&mut self, ctx: &WorkspaceContext) -> Result<(), String> {
        if ctx.current_era != PhaseEra::SixD {
            return Err(format!("Incompatible phase era. Expected SixD, found {:?}", ctx.current_era));
        }

        // Initialize the rusqlite database connection string safely
        let db_path = "hox.db"; 
        let hox_db = HoxRegistry::new(db_path)
            .map_err(|e| format!("Failed to open rusqlite hox.db connection context: {}", e))?;

        self.inner_registry = Some(hox_db);
        self.is_initialized = true;
        Ok(())
    }

    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        if !self.is_initialized {
            return None;
        }

        let db = self.inner_registry.as_ref()?;
        
        // Match directly against the verified get_capability rusqlite method signature
        match db.get_capability(id) {
            Ok(Some(cap)) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                Some(EntityInfo {
                    id: id.to_string(),
                    name: Some(cap.name),
                    version: Some(cap.enzyme_hash), // Map the unique enzyme hash string
                    health: EntryHealth::Healthy,
                    last_seen: now,
                })
            },
            Ok(None) => None,
            Err(_) => Some(EntityInfo {
                id: id.to_string(),
                name: None,
                version: None,
                health: EntryHealth::Failed,
                last_seen: 0,
            }),
        }
    }

    fn list_entities(&self) -> Vec<EntityInfo> {
        if !self.is_initialized {
            return Vec::new();
        }

        let db = match &self.inner_registry {
            Some(registry) => registry,
            None => return Vec::new(),
        };

        match db.list_capabilities() {
            Ok(capabilities) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                capabilities.into_iter()
                    .map(|cap| EntityInfo {
                        id: cap.name.clone(),
                        name: Some(cap.name),
                        version: Some(cap.enzyme_hash),
                        health: EntryHealth::Healthy,
                        last_seen: now,
                    })
                    .collect()
            },
            Err(e) => {
                eprintln!("Error listing Hox capabilities: {}", e);
                Vec::new()
            }
        }
    }

    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        if !self.is_initialized {
            return Err("HoxCapabilityRegistryAdapter: Cannot sync uninitialized database context.".to_string());
        }
        // Base sqlite read verification is stable via query routines; connection context is checked
        Ok(())
    }

    fn registry_type(&self) -> RegistryType {
        self.registry_type
    }
}
