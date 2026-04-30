use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication provider trait
#[async_trait::async_trait]
pub trait AuthProvider: Send + Sync {
    /// Authenticate a request
    async fn authenticate(&self, credentials: &str) -> Result<AuthToken, String>;

    /// Verify token is still valid
    async fn verify_token(&self, token: &AuthToken) -> Result<bool, String>;

    /// Get auth provider type
    fn provider_type(&self) -> &str;
}

/// Authentication token
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthToken {
    /// Token value
    pub token: String,
    /// Token type (Bearer, ApiKey, etc.)
    pub token_type: String,
    /// User/principal ID
    pub principal_id: String,
    /// Scopes/permissions
    pub scopes: Vec<String>,
    /// Token expiration (Unix ms)
    pub expires_at: i64,
    /// Custom claims
    pub claims: HashMap<String, serde_json::Value>,
}

impl AuthToken {
    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp_millis() > self.expires_at
    }

    /// Check if token has required scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.contains(&scope.to_string())
    }
}

/// API Key authentication
pub struct ApiKeyAuth {
    valid_keys: HashMap<String, ApiKeyInfo>,
}

/// API key information
#[derive(Clone, Debug)]
struct ApiKeyInfo {
    principal_id: String,
    scopes: Vec<String>,
    active: bool,
}

impl ApiKeyAuth {
    /// Create new API key authenticator
    pub fn new() -> Self {
        Self {
            valid_keys: HashMap::new(),
        }
    }

    /// Register an API key
    pub fn register_key(&mut self, key: impl Into<String>, principal_id: impl Into<String>, scopes: Vec<String>) {
        let info = ApiKeyInfo {
            principal_id: principal_id.into(),
            scopes,
            active: true,
        };
        self.valid_keys.insert(key.into(), info);
    }

    /// Revoke an API key
    pub fn revoke_key(&mut self, key: &str) {
        if let Some(info) = self.valid_keys.get_mut(key) {
            info.active = false;
        }
    }
}

#[async_trait::async_trait]
impl AuthProvider for ApiKeyAuth {
    async fn authenticate(&self, credentials: &str) -> Result<AuthToken, String> {
        // Extract API key from "Bearer <key>" or just "<key>"
        let key = if credentials.starts_with("Bearer ") {
            &credentials[7..]
        } else {
            credentials
        };

        let info = self.valid_keys
            .get(key)
            .ok_or("Invalid API key")?;

        if !info.active {
            return Err("API key revoked".to_string());
        }

        Ok(AuthToken {
            token: key.to_string(),
            token_type: "ApiKey".to_string(),
            principal_id: info.principal_id.clone(),
            scopes: info.scopes.clone(),
            expires_at: i64::MAX, // API keys don't expire by default
            claims: HashMap::new(),
        })
    }

    async fn verify_token(&self, token: &AuthToken) -> Result<bool, String> {
        if token.is_expired() {
            return Ok(false);
        }

        let info = self.valid_keys
            .get(&token.token)
            .ok_or("Token not found")?;

        Ok(info.active)
    }

    fn provider_type(&self) -> &str {
        "api_key"
    }
}

impl Default for ApiKeyAuth {
    fn default() -> Self {
        Self::new()
    }
}

/// OAuth2 authentication
pub struct OAuth2Auth {
    issuer: String,
    client_id: String,
    client_secret: String,
    // In production, would have JWK set fetching and token verification
}

impl OAuth2Auth {
    /// Create new OAuth2 authenticator
    pub fn new(issuer: impl Into<String>, client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            issuer: issuer.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        }
    }
}

#[async_trait::async_trait]
impl AuthProvider for OAuth2Auth {
    async fn authenticate(&self, credentials: &str) -> Result<AuthToken, String> {
        // In production, would validate JWT signature against issuer's JWK set
        // For now, parse JWT and extract claims
        
        // Simplified: just verify format
        if !credentials.starts_with("Bearer ") {
            return Err("Invalid OAuth2 token format".to_string());
        }

        let token_str = &credentials[7..];
        
        // In production: decode JWT and verify signature
        Ok(AuthToken {
            token: token_str.to_string(),
            token_type: "Bearer".to_string(),
            principal_id: "user-123".to_string(),
            scopes: vec!["read:federation".to_string(), "write:consensus".to_string()],
            expires_at: chrono::Utc::now().timestamp_millis() + 3600000, // 1 hour
            claims: HashMap::new(),
        })
    }

    async fn verify_token(&self, token: &AuthToken) -> Result<bool, String> {
        if token.is_expired() {
            return Ok(false);
        }

        // In production: verify JWT signature
        Ok(true)
    }

    fn provider_type(&self) -> &str {
        "oauth2"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_token_expiration() {
        let token = AuthToken {
            token: "test".to_string(),
            token_type: "Bearer".to_string(),
            principal_id: "user-1".to_string(),
            scopes: vec!["read".to_string()],
            expires_at: 0, // Already expired
            claims: HashMap::new(),
        };

        assert!(token.is_expired());
    }

    #[test]
    fn test_auth_token_scopes() {
        let token = AuthToken {
            token: "test".to_string(),
            token_type: "Bearer".to_string(),
            principal_id: "user-1".to_string(),
            scopes: vec!["read:federation".to_string()],
            expires_at: i64::MAX,
            claims: HashMap::new(),
        };

        assert!(token.has_scope("read:federation"));
        assert!(!token.has_scope("write:consensus"));
    }

    #[tokio::test]
    async fn test_api_key_auth() {
        let mut auth = ApiKeyAuth::new();
        auth.register_key("key-123", "user-1", vec!["read:all".to_string()]);

        let result = auth.authenticate("Bearer key-123").await;
        assert!(result.is_ok());

        let token = result.unwrap();
        assert_eq!(token.principal_id, "user-1");
    }

    #[tokio::test]
    async fn test_api_key_revocation() {
        let mut auth = ApiKeyAuth::new();
        auth.register_key("key-123", "user-1", vec!["read:all".to_string()]);
        
        auth.revoke_key("key-123");

        let result = auth.authenticate("Bearer key-123").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_oauth2_auth() {
        let auth = OAuth2Auth::new("https://auth.example.com", "client-id", "secret");
        
        let result = auth.authenticate("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...").await;
        assert!(result.is_ok());
    }
}
