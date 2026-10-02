/// Role-Based Access Control (RBAC)
///
/// Manage permissions and access rights
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Permission {
    ProposalCreate,
    ProposalView,
    DecisionCreate,
    DecisionExecute,
    SpecialistAccess,
    AuditRead,
    ConfigChange,
    Admin,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Operator,
    Viewer,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub token_id: String,
    pub user_id: String,
    pub role: Role,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}

impl AuthToken {
    pub fn new(user_id: String, role: Role) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            token_id: uuid::Uuid::new_v4().to_string(),
            user_id,
            role,
            issued_at_ms: now,
            expires_at_ms: now + (24 * 60 * 60 * 1000), // 24 hours
        }
    }

    pub fn is_valid(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now < self.expires_at_ms
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControl {
    pub role_permissions: HashMap<Role, HashSet<Permission>>,
    pub tokens: HashMap<String, AuthToken>,
}

impl AccessControl {
    pub fn new() -> Self {
        let mut ac = Self {
            role_permissions: HashMap::new(),
            tokens: HashMap::new(),
        };

        // Set default permissions
        let mut admin_perms = HashSet::new();
        admin_perms.insert(Permission::Admin);
        ac.role_permissions.insert(Role::Admin, admin_perms);

        let mut operator_perms = HashSet::new();
        operator_perms.insert(Permission::ProposalCreate);
        operator_perms.insert(Permission::ProposalView);
        operator_perms.insert(Permission::DecisionExecute);
        operator_perms.insert(Permission::SpecialistAccess);
        ac.role_permissions.insert(Role::Operator, operator_perms);

        let mut viewer_perms = HashSet::new();
        viewer_perms.insert(Permission::ProposalView);
        viewer_perms.insert(Permission::AuditRead);
        ac.role_permissions.insert(Role::Viewer, viewer_perms);

        ac
    }

    /// Issue a token
    pub fn issue_token(&mut self, user_id: String, role: Role) -> AuthToken {
        let token = AuthToken::new(user_id, role);
        self.tokens.insert(token.token_id.clone(), token.clone());
        token
    }

    /// Check authorization
    pub fn authorize(&self, token: &AuthToken, permission_str: &str) -> Result<bool, String> {
        // Check token validity
        if !token.is_valid() {
            return Err("Token expired".to_string());
        }

        // Check if admin
        if matches!(token.role, Role::Admin) {
            return Ok(true);
        }

        // Parse permission
        let permission = match permission_str {
            "proposal_create" => Permission::ProposalCreate,
            "proposal_view" => Permission::ProposalView,
            "decision_create" => Permission::DecisionCreate,
            "decision_execute" => Permission::DecisionExecute,
            "specialist_access" => Permission::SpecialistAccess,
            "audit_read" => Permission::AuditRead,
            "config_change" => Permission::ConfigChange,
            _ => return Err(format!("Unknown permission: {}", permission_str)),
        };

        // Check permissions
        let perms = self.role_permissions.get(&token.role);
        Ok(perms.map(|p| p.contains(&permission)).unwrap_or(false))
    }

    /// Revoke token
    pub fn revoke_token(&mut self, token_id: &str) {
        self.tokens.remove(token_id);
    }
}

impl Default for AccessControl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_token_creation() {
        let token = AuthToken::new("user-1".to_string(), Role::Operator);
        assert!(token.is_valid());
    }

    #[test]
    fn test_access_control_authorize() {
        let ac = AccessControl::new();
        let token = AuthToken::new("user-1".to_string(), Role::Admin);
        let result = ac.authorize(&token, "decision_execute");
        assert!(result.is_ok());
    }
}
