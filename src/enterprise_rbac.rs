/// Enterprise Role-Based Access Control (RBAC) Framework
///
/// Comprehensive RBAC system for fine-grained permission management:
/// - Dynamic role creation and modification
/// - Permission inheritance and composition
/// - Resource-level access control
/// - Tenant-aware policy enforcement
/// - Audit trail of all permission changes

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tracing::{debug, info};

use crate::enterprise_auth::{Permission, Role, User};

/// Custom role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRole {
    pub id: String,
    pub name: String,
    pub description: String,
    pub permissions: Vec<Permission>,
    pub tenant_id: String,
    pub created_by: String,
    pub is_active: bool,
}

impl CustomRole {
    pub fn new(
        id: String,
        name: String,
        description: String,
        tenant_id: String,
        created_by: String,
    ) -> Self {
        Self {
            id,
            name,
            description,
            permissions: Vec::new(),
            tenant_id,
            created_by,
            is_active: true,
        }
    }

    pub fn add_permission(&mut self, permission: Permission) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
        }
    }

    pub fn remove_permission(&mut self, permission: Permission) {
        self.permissions.retain(|p| p != &permission);
    }

    pub fn has_permission(&self, permission: Permission) -> bool {
        self.is_active && self.permissions.contains(&permission)
    }
}

/// Resource-level access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAccess {
    pub resource_id: String,
    pub resource_type: ResourceType,
    pub owner_id: String,
    pub tenant_id: String,
    pub permissions: HashMap<String, HashSet<Permission>>, // user_id -> permissions
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceType {
    Task,
    Memory,
    Specialist,
    Report,
    Configuration,
}

impl ResourceAccess {
    pub fn new(
        resource_id: String,
        resource_type: ResourceType,
        owner_id: String,
        tenant_id: String,
    ) -> Self {
        let mut permissions = HashMap::new();
        
        // Owner gets full access
        let mut owner_perms = HashSet::new();
        owner_perms.insert(Permission::ReadMemory);
        owner_perms.insert(Permission::WriteMemory);
        owner_perms.insert(Permission::DeleteMemory);
        permissions.insert(owner_id.clone(), owner_perms);

        Self {
            resource_id,
            resource_type,
            owner_id,
            tenant_id,
            permissions,
        }
    }

    pub fn grant_permission(
        &mut self,
        user_id: String,
        permission: Permission,
    ) {
        self.permissions
            .entry(user_id)
            .or_insert_with(HashSet::new)
            .insert(permission);
    }

    pub fn revoke_permission(&mut self, user_id: &str, permission: Permission) {
        if let Some(perms) = self.permissions.get_mut(user_id) {
            perms.remove(&permission);
        }
    }

    pub fn has_permission(&self, user_id: &str, permission: Permission) -> bool {
        self.permissions
            .get(user_id)
            .map(|p| p.contains(&permission))
            .unwrap_or(false)
    }

    pub fn share_with(
        &mut self,
        user_id: String,
        permissions: Vec<Permission>,
    ) {
        let mut user_perms = HashSet::new();
        for perm in permissions {
            user_perms.insert(perm);
        }
        self.permissions.insert(user_id, user_perms);
    }
}

/// Policy for automated permission management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tenant_id: String,
    pub rules: Vec<PolicyRule>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub condition: String, // e.g., "role==Operator AND department==Engineering"
    pub action: String,    // e.g., "grant_read_access"
    pub resources: Vec<String>, // Affected resource types
}

impl AccessPolicy {
    pub fn new(
        id: String,
        name: String,
        description: String,
        tenant_id: String,
    ) -> Self {
        Self {
            id,
            name,
            description,
            tenant_id,
            rules: Vec::new(),
            is_active: true,
        }
    }

    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
    }

    pub fn evaluate(&self, user: &User) -> Vec<String> {
        self.rules
            .iter()
            .filter(|rule| self.matches_condition(&rule.condition, user))
            .map(|rule| rule.action.clone())
            .collect()
    }

    fn matches_condition(&self, condition: &str, user: &User) -> bool {
        // Simple condition matching (in production, use expression evaluator)
        condition.contains(&format!("{:?}", user.role)) || condition.contains("*")
    }
}

/// RBAC Manager - orchestrates all role and permission management
pub struct RBACManager {
    custom_roles: Arc<RwLock<HashMap<String, CustomRole>>>,
    resource_access: Arc<RwLock<HashMap<String, ResourceAccess>>>,
    policies: Arc<RwLock<HashMap<String, AccessPolicy>>>,
}

impl RBACManager {
    pub fn new() -> Self {
        Self {
            custom_roles: Arc::new(RwLock::new(HashMap::new())),
            resource_access: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a custom role
    pub fn create_custom_role(&self, role: CustomRole) -> Result<String, String> {
        let mut roles = self.custom_roles.write().unwrap();
        
        if roles.contains_key(&role.id) {
            return Err("Role already exists".to_string());
        }

        let role_id = role.id.clone();
        roles.insert(role_id.clone(), role);
        info!("Custom role created: {}", role_id);
        Ok(role_id)
    }

    /// Get a custom role
    pub fn get_custom_role(&self, role_id: &str) -> Option<CustomRole> {
        let roles = self.custom_roles.read().unwrap();
        roles.get(role_id).cloned()
    }

    /// Update custom role permissions
    pub fn grant_permission_to_role(
        &self,
        role_id: &str,
        permission: Permission,
    ) -> Result<(), String> {
        let mut roles = self.custom_roles.write().unwrap();
        if let Some(role) = roles.get_mut(role_id) {
            role.add_permission(permission);
            info!("Permission {:?} granted to role: {}", permission, role_id);
            Ok(())
        } else {
            Err("Role not found".to_string())
        }
    }

    /// Register resource-level access
    pub fn register_resource(&self, access: ResourceAccess) -> Result<String, String> {
        let mut resources = self.resource_access.write().unwrap();
        
        if resources.contains_key(&access.resource_id) {
            return Err("Resource already registered".to_string());
        }

        let resource_id = access.resource_id.clone();
        resources.insert(resource_id.clone(), access);
        info!("Resource registered: {}", resource_id);
        Ok(resource_id)
    }

    /// Share resource with user
    pub fn share_resource(
        &self,
        resource_id: &str,
        user_id: String,
        permissions: Vec<Permission>,
    ) -> Result<(), String> {
        let mut resources = self.resource_access.write().unwrap();
        if let Some(resource) = resources.get_mut(resource_id) {
            resource.share_with(user_id.clone(), permissions);
            info!("Resource {} shared with user: {}", resource_id, user_id);
            Ok(())
        } else {
            Err("Resource not found".to_string())
        }
    }

    /// Check resource access
    pub fn check_resource_access(
        &self,
        resource_id: &str,
        user_id: &str,
        permission: Permission,
    ) -> Result<(), String> {
        let resources = self.resource_access.read().unwrap();
        if let Some(resource) = resources.get(resource_id) {
            if resource.has_permission(user_id, permission) {
                Ok(())
            } else {
                Err("Access denied".to_string())
            }
        } else {
            Err("Resource not found".to_string())
        }
    }

    /// Create access policy
    pub fn create_policy(&self, policy: AccessPolicy) -> Result<String, String> {
        let mut policies = self.policies.write().unwrap();
        
        if policies.contains_key(&policy.id) {
            return Err("Policy already exists".to_string());
        }

        let policy_id = policy.id.clone();
        policies.insert(policy_id.clone(), policy);
        info!("Access policy created: {}", policy_id);
        Ok(policy_id)
    }

    /// Evaluate policies for user
    pub fn evaluate_policies(&self, user: &User) -> Vec<String> {
        let policies = self.policies.read().unwrap();
        policies
            .values()
            .filter(|p| p.is_active && p.tenant_id == user.tenant_id)
            .flat_map(|p| p.evaluate(user))
            .collect()
    }

    /// List all custom roles for tenant
    pub fn list_roles(&self, tenant_id: &str) -> Vec<CustomRole> {
        let roles = self.custom_roles.read().unwrap();
        roles
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.is_active)
            .cloned()
            .collect()
    }

    /// Get all resources for user
    pub fn list_user_resources(&self, user_id: &str) -> Vec<ResourceAccess> {
        let resources = self.resource_access.read().unwrap();
        resources
            .values()
            .filter(|r| r.permissions.contains_key(user_id))
            .cloned()
            .collect()
    }
}

impl Default for RBACManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_role_creation() {
        let role = CustomRole::new(
            "analyst".to_string(),
            "Data Analyst".to_string(),
            "Can analyze data".to_string(),
            "tenant-1".to_string(),
            "admin-1".to_string(),
        );

        assert_eq!(role.name, "Data Analyst");
        assert!(role.is_active);
    }

    #[test]
    fn test_custom_role_permissions() {
        let mut role = CustomRole::new(
            "analyst".to_string(),
            "Data Analyst".to_string(),
            "Can analyze data".to_string(),
            "tenant-1".to_string(),
            "admin-1".to_string(),
        );

        role.add_permission(Permission::ReadMemory);
        assert!(role.has_permission(Permission::ReadMemory));
        assert!(!role.has_permission(Permission::WriteMemory));
    }

    #[test]
    fn test_resource_access_creation() {
        let resource = ResourceAccess::new(
            "task-1".to_string(),
            ResourceType::Task,
            "user-1".to_string(),
            "tenant-1".to_string(),
        );

        assert!(resource.has_permission("user-1", Permission::WriteMemory));
    }

    #[test]
    fn test_resource_access_sharing() {
        let mut resource = ResourceAccess::new(
            "task-1".to_string(),
            ResourceType::Task,
            "user-1".to_string(),
            "tenant-1".to_string(),
        );

        resource.share_with("user-2".to_string(), vec![Permission::ReadMemory]);
        assert!(resource.has_permission("user-2", Permission::ReadMemory));
    }

    #[test]
    fn test_rbac_manager_create_role() {
        let manager = RBACManager::new();
        let role = CustomRole::new(
            "analyst".to_string(),
            "Data Analyst".to_string(),
            "Can analyze data".to_string(),
            "tenant-1".to_string(),
            "admin-1".to_string(),
        );

        assert!(manager.create_custom_role(role).is_ok());
    }

    #[test]
    fn test_rbac_manager_grant_permission() {
        let manager = RBACManager::new();
        let role = CustomRole::new(
            "analyst".to_string(),
            "Data Analyst".to_string(),
            "Can analyze data".to_string(),
            "tenant-1".to_string(),
            "admin-1".to_string(),
        );

        manager.create_custom_role(role).ok();
        assert!(manager.grant_permission_to_role("analyst", Permission::ReadMemory).is_ok());
    }

    #[test]
    fn test_rbac_manager_resource_sharing() {
        let manager = RBACManager::new();
        let resource = ResourceAccess::new(
            "task-1".to_string(),
            ResourceType::Task,
            "user-1".to_string(),
            "tenant-1".to_string(),
        );

        manager.register_resource(resource).ok();
        assert!(manager
            .share_resource("task-1", "user-2".to_string(), vec![Permission::ReadMemory])
            .is_ok());
    }

    #[test]
    fn test_rbac_manager_check_access() {
        let manager = RBACManager::new();
        let resource = ResourceAccess::new(
            "task-1".to_string(),
            ResourceType::Task,
            "user-1".to_string(),
            "tenant-1".to_string(),
        );

        manager.register_resource(resource).ok();
        assert!(manager.check_resource_access("task-1", "user-1", Permission::WriteMemory).is_ok());
        assert!(manager.check_resource_access("task-1", "user-2", Permission::WriteMemory).is_err());
    }

    #[test]
    fn test_access_policy_creation() {
        let policy = AccessPolicy::new(
            "policy-1".to_string(),
            "Engineering Policy".to_string(),
            "Policy for engineering team".to_string(),
            "tenant-1".to_string(),
        );

        assert_eq!(policy.name, "Engineering Policy");
        assert!(policy.is_active);
    }

    #[test]
    fn test_rbac_manager_list_roles() {
        let manager = RBACManager::new();
        let role = CustomRole::new(
            "analyst".to_string(),
            "Data Analyst".to_string(),
            "Can analyze data".to_string(),
            "tenant-1".to_string(),
            "admin-1".to_string(),
        );

        manager.create_custom_role(role).ok();
        let roles = manager.list_roles("tenant-1");
        assert_eq!(roles.len(), 1);
    }

    #[test]
    fn test_rbac_manager_list_user_resources() {
        let manager = RBACManager::new();
        let resource = ResourceAccess::new(
            "task-1".to_string(),
            ResourceType::Task,
            "user-1".to_string(),
            "tenant-1".to_string(),
        );

        manager.register_resource(resource).ok();
        let resources = manager.list_user_resources("user-1");
        assert_eq!(resources.len(), 1);
    }
}
