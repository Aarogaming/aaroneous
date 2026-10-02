/// Security Hardening Module
///
/// TLS configuration, encryption, and security policies
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TLSConfig {
    pub enabled: bool,
    pub cert_path: String,
    pub key_path: String,
    pub min_tls_version: String,
    pub ciphers: Vec<String>,
}

impl Default for TLSConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cert_path: "/etc/aaroneous/cert.pem".to_string(),
            key_path: "/etc/aaroneous/key.pem".to_string(),
            min_tls_version: "1.2".to_string(),
            ciphers: vec![
                "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string(),
                "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEncryption {
    pub encrypt_at_rest: bool,
    pub encrypt_in_transit: bool,
    pub key_rotation_days: u32,
    pub algorithm: String,
}

impl Default for DataEncryption {
    fn default() -> Self {
        Self {
            encrypt_at_rest: true,
            encrypt_in_transit: true,
            key_rotation_days: 90,
            algorithm: "AES-256-GCM".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub tls: TLSConfig,
    pub encryption: DataEncryption,
    pub require_auth: bool,
    pub session_timeout_minutes: u32,
    pub max_failed_attempts: u32,
    pub lockout_duration_minutes: u32,
}

impl SecurityConfig {
    pub fn strict() -> Self {
        Self {
            tls: TLSConfig::default(),
            encryption: DataEncryption::default(),
            require_auth: true,
            session_timeout_minutes: 15,
            max_failed_attempts: 3,
            lockout_duration_minutes: 30,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self::strict()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_default() {
        let config = TLSConfig::default();
        assert!(config.enabled);
    }

    #[test]
    fn test_security_config_strict() {
        let config = SecurityConfig::strict();
        assert!(config.require_auth);
        assert_eq!(config.session_timeout_minutes, 15);
    }
}
