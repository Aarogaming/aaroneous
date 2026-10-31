use super::capability::CapabilityClass;
use crate::capability_broker::SandboxPolicy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic clock abstraction for time-gated authorization and zero-ambient time enforcement.
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// System-time-derived clock for production service initialization.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Deterministic mock clock for tests and simulated replay.
#[derive(Debug, Clone)]
pub struct MockClock {
    current: Arc<AtomicU64>,
}

impl MockClock {
    pub fn new(initial_ms: u64) -> Self {
        Self {
            current: Arc::new(AtomicU64::new(initial_ms)),
        }
    }

    pub fn advance(&self, ms: u64) {
        self.current.fetch_add(ms, Ordering::SeqCst);
    }

    pub fn set(&self, ms: u64) {
        self.current.store(ms, Ordering::SeqCst);
    }
}

impl Clock for MockClock {
    fn now_ms(&self) -> u64 {
        self.current.load(Ordering::SeqCst)
    }
}

/// Explicit authorization errors for capability dispatch and caller verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorizationError {
    MissingCredentials,
    InvalidCredentials(String),
    Expired {
        now_ms: u64,
        expires_at_ms: u64,
    },
    Revoked(String),
    InsufficientCapabilityClass {
        required: CapabilityClass,
        granted: CapabilityClass,
    },
    ScopeDisallowed {
        tool_name: String,
        granted_scopes: Vec<String>,
    },
    SandboxViolation(String),
    PrincipalMismatch {
        expected: String,
        presented: String,
    },
}

impl std::fmt::Display for AuthorizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCredentials => write!(f, "Missing authentication credentials"),
            Self::InvalidCredentials(msg) => write!(f, "Invalid credentials: {msg}"),
            Self::Expired {
                now_ms,
                expires_at_ms,
            } => write!(
                f,
                "Authorization expired (now: {now_ms}ms, expired_at: {expires_at_ms}ms)"
            ),
            Self::Revoked(msg) => write!(f, "Authorization revoked: {msg}"),
            Self::InsufficientCapabilityClass { required, granted } => write!(
                f,
                "Insufficient capability class (required: {required:?}, granted: {granted:?})"
            ),
            Self::ScopeDisallowed {
                tool_name,
                granted_scopes,
            } => write!(
                f,
                "Tool '{tool_name}' not granted by scopes: {granted_scopes:?}"
            ),
            Self::SandboxViolation(msg) => write!(f, "Sandbox policy violation: {msg}"),
            Self::PrincipalMismatch {
                expected,
                presented,
            } => write!(
                f,
                "Principal mismatch (expected: {expected}, presented: {presented})"
            ),
        }
    }
}

impl std::error::Error for AuthorizationError {}

/// Trusted caller context derived strictly from authenticated transport credentials.
///
/// Caller-supplied principal or scope claims in request bodies are NEVER trusted;
/// authorization is strictly evaluated against this validated context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallerContext {
    /// Authenticated principal identity (e.g., "controller-1", "desktop-client")
    pub principal_id: String,
    /// Maximum allowed capability class for this caller
    pub max_capability_class: CapabilityClass,
    /// Scoped capability patterns granted to this caller (e.g. `["knowledge.*", "security.audit"]`)
    pub allowed_scopes: Vec<String>,
    /// Sandbox security isolation policy
    pub sandbox_policy: SandboxPolicy,
    /// Issue timestamp in monotonic Unix ms
    pub issued_at_ms: u64,
    /// Expiration timestamp in monotonic Unix ms
    pub expires_at_ms: u64,
    /// Whether this principal or token has been revoked
    pub is_revoked: bool,
    /// Unique token identifier
    pub token_id: Option<uuid::Uuid>,
}

impl CallerContext {
    /// Check whether the caller context has expired at the given monotonic Unix ms.
    pub fn is_expired_at(&self, now_ms: u64) -> bool {
        now_ms > self.expires_at_ms
    }

    /// Verify whether this caller context is authorized to invoke a tool of `required_class` at `now_ms`.
    pub fn is_authorized_for(
        &self,
        tool_name: &str,
        required_class: CapabilityClass,
        now_ms: u64,
    ) -> Result<(), AuthorizationError> {
        if self.is_revoked {
            return Err(AuthorizationError::Revoked(format!(
                "Principal '{}' or token is revoked",
                self.principal_id
            )));
        }

        if self.is_expired_at(now_ms) {
            return Err(AuthorizationError::Expired {
                now_ms,
                expires_at_ms: self.expires_at_ms,
            });
        }

        // 1. Capability class hierarchy check: caller cannot execute above max_capability_class
        if self.max_capability_class < required_class {
            return Err(AuthorizationError::InsufficientCapabilityClass {
                required: required_class,
                granted: self.max_capability_class,
            });
        }

        // 2. Sandbox policy check
        if self.sandbox_policy == SandboxPolicy::Airgapped {
            if tool_name.starts_with("platform.")
                || tool_name.starts_with("screen.")
                || tool_name.starts_with("audio.")
                || tool_name.starts_with("code.process")
            {
                return Err(AuthorizationError::SandboxViolation(format!(
                    "Airgapped sandbox disallows tool '{tool_name}'"
                )));
            }
        } else if self.sandbox_policy == SandboxPolicy::ReadConstrained
            && required_class == CapabilityClass::Mutation
        {
            return Err(AuthorizationError::SandboxViolation(format!(
                "Read-constrained sandbox disallows mutating tool '{tool_name}'"
            )));
        }

        // 3. Scopes match check
        let permitted = self.allowed_scopes.iter().any(|s| {
            if s == "*" {
                // Wildcard grant permits tools up to the caller's max_capability_class
                true
            } else if s == tool_name {
                true
            } else if let Some(prefix) = s.strip_suffix(".*") {
                if let Some(rest) = tool_name.strip_prefix(prefix) {
                    rest.starts_with('.')
                } else {
                    false
                }
            } else {
                false
            }
        });

        if !permitted {
            return Err(AuthorizationError::ScopeDisallowed {
                tool_name: tool_name.to_string(),
                granted_scopes: self.allowed_scopes.clone(),
            });
        }

        Ok(())
    }

    /// Explicit local development compatibility adapter (bounded to Verification, ReadConstrained sandbox).
    pub fn local_development(now_ms: u64) -> Self {
        Self {
            principal_id: "local-operator".to_string(),
            max_capability_class: CapabilityClass::Verification,
            allowed_scopes: vec!["*".to_string()],
            sandbox_policy: SandboxPolicy::ReadConstrained,
            issued_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(3_600_000), // 1 hour
            is_revoked: false,
            token_id: Some(uuid::Uuid::nil()),
        }
    }
}

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
    /// Check if token is expired at given timestamp
    pub fn is_expired_at(&self, now_ms: i64) -> bool {
        now_ms > self.expires_at
    }

    /// Check if token has required scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.contains(&scope.to_string())
    }
}

/// API Key authentication
pub struct ApiKeyAuth {
    valid_keys: HashMap<String, ApiKeyInfo>,
    revoked_principals: HashSet<String>,
    revoked_tokens: HashSet<uuid::Uuid>,
    clock: Arc<dyn Clock>,
}

/// API key information
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiKeyInfo {
    pub principal_id: String,
    pub max_capability_class: CapabilityClass,
    pub scopes: Vec<String>,
    pub sandbox_policy: SandboxPolicy,
    pub active: bool,
    pub expires_at_ms: Option<u64>,
    pub token_id: Option<uuid::Uuid>,
}

fn hash_key(key: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

impl ApiKeyAuth {
    /// Create new API key authenticator
    pub fn new() -> Self {
        Self::with_clock(Arc::new(SystemClock))
    }

    /// Create new API key authenticator with injected clock
    pub fn with_clock(clock: Arc<dyn Clock>) -> Self {
        Self {
            valid_keys: HashMap::new(),
            revoked_principals: HashSet::new(),
            revoked_tokens: HashSet::new(),
            clock,
        }
    }

    /// Returns true if no API keys are registered
    pub fn is_empty(&self) -> bool {
        self.valid_keys.is_empty()
    }

    /// Register an API key with default Verification class and ReadConstrained sandbox
    pub fn register_key(
        &mut self,
        key: impl Into<String>,
        principal_id: impl Into<String>,
        scopes: Vec<String>,
    ) {
        self.register_key_with_class(
            key,
            principal_id,
            CapabilityClass::Verification,
            scopes,
            SandboxPolicy::ReadConstrained,
        );
    }

    /// Register an API key with explicit capability class and sandbox policy
    pub fn register_key_with_class(
        &mut self,
        key: impl Into<String>,
        principal_id: impl Into<String>,
        max_capability_class: CapabilityClass,
        scopes: Vec<String>,
        sandbox_policy: SandboxPolicy,
    ) {
        self.register_key_with_expiry(
            key,
            principal_id,
            max_capability_class,
            scopes,
            sandbox_policy,
            None,
        );
    }

    /// Register an API key with explicit capability class, sandbox policy, and optional expiration
    pub fn register_key_with_expiry(
        &mut self,
        key: impl Into<String>,
        principal_id: impl Into<String>,
        max_capability_class: CapabilityClass,
        scopes: Vec<String>,
        sandbox_policy: SandboxPolicy,
        expires_at_ms: Option<u64>,
    ) {
        let key_str = key.into();
        let key_hash = hash_key(&key_str);
        let info = ApiKeyInfo {
            principal_id: principal_id.into(),
            max_capability_class,
            scopes,
            sandbox_policy,
            active: true,
            expires_at_ms,
            token_id: None,
        };
        self.valid_keys.insert(key_hash, info);
    }

    /// Register an API key with explicit capability class, scopes, sandbox policy, and token ID
    pub fn register_key_with_token(
        &mut self,
        key: impl Into<String>,
        principal_id: impl Into<String>,
        max_capability_class: CapabilityClass,
        scopes: Vec<String>,
        token_id: uuid::Uuid,
    ) {
        let key_str = key.into();
        let key_hash = hash_key(&key_str);
        let info = ApiKeyInfo {
            principal_id: principal_id.into(),
            max_capability_class,
            scopes,
            sandbox_policy: SandboxPolicy::FullPrivilege,
            active: true,
            expires_at_ms: None,
            token_id: Some(token_id),
        };
        self.valid_keys.insert(key_hash, info);
    }

    /// Revoke an API key
    pub fn revoke_key(&mut self, key: &str) {
        let key_hash = hash_key(key);
        if let Some(info) = self.valid_keys.get_mut(&key_hash) {
            info.active = false;
        }
    }

    /// Revoke a principal across all tokens
    pub fn revoke_principal(&mut self, principal_id: &str) {
        self.revoked_principals.insert(principal_id.to_string());
    }

    /// Revoke an explicit token ID
    pub fn revoke_token(&mut self, token_id: &uuid::Uuid) {
        self.revoked_tokens.insert(*token_id);
    }

    /// Check if a principal is revoked
    pub fn is_principal_revoked(&self, principal_id: &str) -> bool {
        self.revoked_principals.contains(principal_id)
    }

    /// Check if a token ID is revoked
    pub fn is_token_revoked(&self, token_id: &uuid::Uuid) -> bool {
        self.revoked_tokens.contains(token_id)
    }

    /// Check whether a caller context or its associated token is revoked
    pub fn is_context_revoked(&self, context: &CallerContext) -> bool {
        if context.is_revoked || self.is_principal_revoked(&context.principal_id) {
            return true;
        }
        context
            .token_id
            .as_ref()
            .is_some_and(|tid| self.is_token_revoked(tid))
    }

    /// Authenticate a request credentials string and derive a validated CallerContext
    pub fn authenticate_caller(
        &self,
        credentials: &str,
        now_ms: u64,
    ) -> Result<CallerContext, AuthorizationError> {
        let key = credentials
            .strip_prefix("Bearer ")
            .or_else(|| credentials.strip_prefix("bearer "))
            .unwrap_or(credentials)
            .trim();

        if key.is_empty() {
            return Err(AuthorizationError::MissingCredentials);
        }

        // Check if the credentials string is a revoked token UUID
        if uuid::Uuid::parse_str(key).is_ok_and(|tid| self.is_token_revoked(&tid)) {
            return Err(AuthorizationError::Revoked(format!(
                "Token '{key}' is revoked"
            )));
        }

        let key_hash = hash_key(key);
        let info = self.valid_keys.get(&key_hash).ok_or_else(|| {
            AuthorizationError::InvalidCredentials("Key not recognized".to_string())
        })?;

        if !info.active {
            return Err(AuthorizationError::Revoked(
                "API key is deactivated".to_string(),
            ));
        }

        if self.is_principal_revoked(&info.principal_id) {
            return Err(AuthorizationError::Revoked(format!(
                "Principal '{}' is revoked",
                info.principal_id
            )));
        }

        if info
            .token_id
            .as_ref()
            .is_some_and(|tid| self.is_token_revoked(tid))
        {
            return Err(AuthorizationError::Revoked(format!(
                "Token '{:?}' is revoked",
                info.token_id
            )));
        }

        if let Some(exp) = info.expires_at_ms.filter(|&exp| now_ms > exp) {
            return Err(AuthorizationError::Expired {
                now_ms,
                expires_at_ms: exp,
            });
        }

        let expires_at_ms = match info.expires_at_ms {
            Some(exp) => exp.min(now_ms.saturating_add(86_400_000)),
            None => now_ms.saturating_add(86_400_000),
        };

        let token_id = info.token_id.unwrap_or_else(uuid::Uuid::new_v4);

        Ok(CallerContext {
            principal_id: info.principal_id.clone(),
            max_capability_class: info.max_capability_class,
            allowed_scopes: info.scopes.clone(),
            sandbox_policy: info.sandbox_policy,
            issued_at_ms: now_ms,
            expires_at_ms,
            is_revoked: false,
            token_id: Some(token_id),
        })
    }
}

#[async_trait::async_trait]
impl AuthProvider for ApiKeyAuth {
    async fn authenticate(&self, credentials: &str) -> Result<AuthToken, String> {
        // Extract API key from "Bearer <key>" or just "<key>"
        let key = credentials
            .strip_prefix("Bearer ")
            .or_else(|| credentials.strip_prefix("bearer "))
            .unwrap_or(credentials)
            .trim();
        let key_hash = hash_key(key);

        let info = self.valid_keys.get(&key_hash).ok_or("Invalid API key")?;

        if !info.active {
            return Err("API key revoked".to_string());
        }

        if self.is_principal_revoked(&info.principal_id) {
            return Err(format!("Principal '{}' is revoked", info.principal_id));
        }

        let now_ms = self.clock.now_ms();
        if info.expires_at_ms.is_some_and(|exp| now_ms > exp) {
            return Err("API key has expired".to_string());
        }

        let expires_at = info.expires_at_ms.map(|e| e as i64).unwrap_or(i64::MAX);

        let token_id = info.token_id.unwrap_or_else(uuid::Uuid::new_v4);
        let mut claims = HashMap::new();
        claims.insert(
            "token_id".to_string(),
            serde_json::Value::String(token_id.to_string()),
        );

        Ok(AuthToken {
            token: key.to_string(),
            token_type: "ApiKey".to_string(),
            principal_id: info.principal_id.clone(),
            scopes: info.scopes.clone(),
            expires_at,
            claims,
        })
    }

    async fn verify_token(&self, token: &AuthToken) -> Result<bool, String> {
        let key_hash = hash_key(&token.token);
        let info = self.valid_keys.get(&key_hash).ok_or("Token not found")?;

        if !info.active {
            return Ok(false);
        }

        if self.is_principal_revoked(&info.principal_id)
            || self.is_principal_revoked(&token.principal_id)
        {
            return Ok(false);
        }

        if info
            .token_id
            .as_ref()
            .is_some_and(|tid| self.is_token_revoked(tid))
        {
            return Ok(false);
        }

        let is_revoked = token
            .claims
            .get("token_id")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .is_some_and(|tid| self.is_token_revoked(&tid));
        if is_revoked {
            return Ok(false);
        }

        // Strict principal match
        if token.principal_id != info.principal_id {
            return Ok(false);
        }

        let now_ms = self.clock.now_ms() as i64;
        // Expiration check against clock
        if token.is_expired_at(now_ms) {
            return Ok(false);
        }

        if info.expires_at_ms.is_some_and(|exp| (now_ms as u64) > exp) {
            return Ok(false);
        }

        Ok(true)
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
    _issuer: String,
    _client_id: String,
    _client_secret: String,
}

impl OAuth2Auth {
    /// Create new OAuth2 authenticator
    pub fn new(
        issuer: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        Self {
            _issuer: issuer.into(),
            _client_id: client_id.into(),
            _client_secret: client_secret.into(),
        }
    }
}

#[async_trait::async_trait]
impl AuthProvider for OAuth2Auth {
    async fn authenticate(&self, credentials: &str) -> Result<AuthToken, String> {
        let _ = credentials;
        Err(
            "OAuth2 authentication is not configured; use a verified provider or ApiKeyAuth"
                .to_string(),
        )
    }

    async fn verify_token(&self, token: &AuthToken) -> Result<bool, String> {
        let _ = token;
        Err("OAuth2 token verification is not configured".to_string())
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
            expires_at: 1000,
            claims: HashMap::new(),
        };

        assert!(token.is_expired_at(1001));
        assert!(!token.is_expired_at(999));
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
    async fn test_principal_revocation() {
        let mut auth = ApiKeyAuth::new();
        auth.register_key("key-123", "user-1", vec!["read:all".to_string()]);

        let caller = auth.authenticate_caller("Bearer key-123", 1000).unwrap();
        assert_eq!(caller.principal_id, "user-1");
        assert!(!caller.is_revoked);

        auth.revoke_principal("user-1");
        let result = auth.authenticate_caller("Bearer key-123", 1000);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AuthorizationError::Revoked(_)
        ));
    }

    #[test]
    fn test_caller_context_authorization_and_least_privilege() {
        let now_ms = 1_000_000;
        let caller = CallerContext {
            principal_id: "auditor".to_string(),
            max_capability_class: CapabilityClass::Verification,
            allowed_scopes: vec!["security.*".to_string(), "knowledge.query".to_string()],
            sandbox_policy: SandboxPolicy::ReadConstrained,
            issued_at_ms: now_ms,
            expires_at_ms: now_ms + 60_000,
            is_revoked: false,
            token_id: Some(uuid::Uuid::new_v4()),
        };

        // Allowed: verification tool in granted scope
        assert!(
            caller
                .is_authorized_for("security.audit", CapabilityClass::Verification, now_ms)
                .is_ok()
        );

        // Allowed: read-only tool in granted scope (Verification class >= ReadOnlyInspection)
        assert!(
            caller
                .is_authorized_for(
                    "knowledge.query",
                    CapabilityClass::ReadOnlyInspection,
                    now_ms
                )
                .is_ok()
        );

        // Rejected: scope not granted
        let ungranted = caller.is_authorized_for(
            "knowledge.unmapped",
            CapabilityClass::ReadOnlyInspection,
            now_ms,
        );
        assert!(ungranted.is_err());
        assert!(matches!(
            ungranted.unwrap_err(),
            AuthorizationError::ScopeDisallowed { .. }
        ));

        // Rejected: capability class requires Mutation, but caller only has Verification
        let mutation =
            caller.is_authorized_for("security.mutate_keys", CapabilityClass::Mutation, now_ms);
        assert!(mutation.is_err());
        assert!(matches!(
            mutation.unwrap_err(),
            AuthorizationError::InsufficientCapabilityClass { .. }
        ));

        // Rejected: expired
        let expired = caller.is_authorized_for(
            "security.audit",
            CapabilityClass::Verification,
            now_ms + 70_000,
        );
        assert!(expired.is_err());
        assert!(matches!(
            expired.unwrap_err(),
            AuthorizationError::Expired { .. }
        ));
    }

    #[test]
    fn test_caller_context_airgapped_sandbox_policy() {
        let now_ms = 1_000_000;
        let caller = CallerContext {
            principal_id: "airgapped-worker".to_string(),
            max_capability_class: CapabilityClass::Mutation,
            allowed_scopes: vec!["*".to_string()],
            sandbox_policy: SandboxPolicy::Airgapped,
            issued_at_ms: now_ms,
            expires_at_ms: now_ms + 60_000,
            is_revoked: false,
            token_id: Some(uuid::Uuid::new_v4()),
        };

        // Rejected: platform / actuation tool disallowed under Airgapped
        let plat = caller.is_authorized_for("platform.reboot", CapabilityClass::Mutation, now_ms);
        assert!(plat.is_err());
        assert!(matches!(
            plat.unwrap_err(),
            AuthorizationError::SandboxViolation(_)
        ));
    }

    #[test]
    fn test_wildcard_scope_namespace_isolation() {
        let now_ms = 1_000_000;
        let caller = CallerContext {
            principal_id: "scoped-caller".to_string(),
            max_capability_class: CapabilityClass::Mutation,
            allowed_scopes: vec!["security.*".to_string()],
            sandbox_policy: SandboxPolicy::FullPrivilege,
            issued_at_ms: now_ms,
            expires_at_ms: now_ms + 60_000,
            is_revoked: false,
            token_id: Some(uuid::Uuid::new_v4()),
        };

        // Allowed: within security namespace
        assert!(
            caller
                .is_authorized_for("security.audit", CapabilityClass::Mutation, now_ms)
                .is_ok()
        );
        assert!(
            caller
                .is_authorized_for("security.scanner", CapabilityClass::Mutation, now_ms)
                .is_ok()
        );

        // Rejected: adjacent namespace without '.' separator
        assert!(
            caller
                .is_authorized_for("securityevil.audit", CapabilityClass::Mutation, now_ms)
                .is_err()
        );
        assert!(
            caller
                .is_authorized_for("security", CapabilityClass::Mutation, now_ms)
                .is_err()
        );
        assert!(
            caller
                .is_authorized_for("security_test", CapabilityClass::Mutation, now_ms)
                .is_err()
        );
        assert!(
            caller
                .is_authorized_for("other.tool", CapabilityClass::Mutation, now_ms)
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_verify_token_principal_and_expiry_validation() {
        let mock_clock = Arc::new(MockClock::new(1000));
        let mut auth = ApiKeyAuth::with_clock(mock_clock.clone());
        auth.register_key_with_expiry(
            "valid-key",
            "alice",
            CapabilityClass::Verification,
            vec!["*".to_string()],
            SandboxPolicy::ReadConstrained,
            Some(5000),
        );

        let mut token = auth.authenticate("Bearer valid-key").await.unwrap();
        assert_eq!(token.principal_id, "alice");

        // Token valid at now=1000
        assert_eq!(auth.verify_token(&token).await, Ok(true));

        // Tampered principal returns Ok(false)
        token.principal_id = "bob".to_string();
        assert_eq!(auth.verify_token(&token).await, Ok(false));

        // Restored principal is valid again
        token.principal_id = "alice".to_string();
        assert_eq!(auth.verify_token(&token).await, Ok(true));

        // Advance clock past expiration (now=6000 > 5000)
        mock_clock.advance(5000);
        assert_eq!(auth.verify_token(&token).await, Ok(false));

        // Authenticate past expiration also fails
        let exp_res = auth.authenticate_caller("Bearer valid-key", 6000);
        assert!(matches!(exp_res, Err(AuthorizationError::Expired { .. })));
    }

    #[tokio::test]
    async fn test_token_revocation_lifecycle() {
        let clock = Arc::new(MockClock::new(1000));
        let mut auth = ApiKeyAuth::with_clock(clock.clone());

        let token_id = uuid::Uuid::new_v4();
        auth.register_key_with_token(
            "revocable-key",
            "revocable-user",
            CapabilityClass::Verification,
            vec!["*".to_string()],
            token_id,
        );

        assert!(!auth.is_token_revoked(&token_id));

        let caller = auth
            .authenticate_caller("Bearer revocable-key", 1000)
            .unwrap();
        assert_eq!(caller.token_id, Some(token_id));
        assert!(!auth.is_context_revoked(&caller));

        let token = auth.authenticate("Bearer revocable-key").await.unwrap();
        assert_eq!(auth.verify_token(&token).await, Ok(true));

        // Revoke the explicit token ID
        auth.revoke_token(&token_id);
        assert!(auth.is_token_revoked(&token_id));

        // verify_token now rejects the revoked token
        assert_eq!(auth.verify_token(&token).await, Ok(false));

        // authenticate_caller rejects the revoked token
        let res = auth.authenticate_caller("Bearer revocable-key", 1000);
        assert!(matches!(res, Err(AuthorizationError::Revoked(_))));

        // is_context_revoked identifies the caller context as revoked
        assert!(auth.is_context_revoked(&caller));

        // Raw UUID as credentials is also rejected as revoked
        let raw_res = auth.authenticate_caller(&token_id.to_string(), 1000);
        assert!(matches!(raw_res, Err(AuthorizationError::Revoked(_))));
    }

    #[tokio::test]
    async fn test_oauth2_auth() {
        let auth = OAuth2Auth::new("https://auth.example.com", "client-id", "secret");

        let result = auth
            .authenticate("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")
            .await;
        assert!(result.is_err());
    }
}
