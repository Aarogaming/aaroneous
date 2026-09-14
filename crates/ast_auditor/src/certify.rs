//! Cryptographic Certification & ABI Layout Hashing for Cratify ACCs.
//!
//! Generates deterministic "Brand Seals" from AST state, audit reports,
//! and manifest metadata.  Computes ABI layout hashes for `bytemuck::Pod`
//! structs to prevent shared-memory ABI corruption.  Provides a verification
//! state machine that checks whether ACC components meet isolation
//! requirements (`max_blast_radius = "isolated"`).

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::Path;

// ── Error Type ────────────────────────────────────────────────────────

/// Errors that can arise during certification.
#[derive(Debug, thiserror::Error)]
pub enum CertError {
    /// Hash computation failed.
    #[error("hash computation failed: {0}")]
    HashError(String),

    /// Signature creation or verification failed.
    #[error("signature error: {0}")]
    SignatureError(String),

    /// ABI layout hash does not match the expected value.
    #[error("ABI layout mismatch (expected {expected}, got {actual})")]
    AbiMismatch { expected: String, actual: String },

    /// The certification seal does not match the expected value.
    #[error("seal mismatch (expected {expected}, got {actual})")]
    SealMismatch { expected: String, actual: String },

    /// Certification metadata is missing from the manifest.
    #[error("missing certification metadata in manifest")]
    MissingMetadata,

    /// The component does not satisfy isolation requirements.
    #[error("isolation violation: {0}")]
    IsolationViolation(String),
}

/// Convenience alias for certification results.
pub type CertResult<T> = std::result::Result<T, CertError>;

// ── Core Data Structures ─────────────────────────────────────────────

/// The computed certification payload — a Brand Seal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Certification {
    /// Hex-encoded SHA-256 of the combined source, audit, and binary hashes.
    pub seal: String,
    /// Base64-encoded ECDSA-P256 signature of `seal` (if signing key provided).
    pub signature: Option<String>,
    /// Hex-encoded ABI layout hash for zero-copy Pod structs.
    pub abi_layout_hash: String,
    /// Unix timestamp (microseconds) when the certification was generated.
    pub created_at_us: u64,
    /// Isolation tier the component was certified into.
    pub isolation_tier: IsolationTier,
}

/// Isolation tiers for ACC components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IsolationTier {
    /// Fully isolated — no cross-component state sharing.
    Isolated,
    /// Shared-nothing but may communicate via IPC.
    Subordinate,
    /// Core system component with elevated privileges.
    Core,
}

impl fmt::Display for IsolationTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Isolated => write!(f, "isolated"),
            Self::Subordinate => write!(f, "subordinate"),
            Self::Core => write!(f, "core"),
        }
    }
}

// ── Seal Generation ──────────────────────────────────────────────────

/// Raw inputs for seal computation.
pub struct SealInput<'a> {
    /// Serialized AST / source fingerprint.
    pub source_ast: &'a [u8],
    /// Serialized audit report (JSON).
    pub audit_report: &'a [u8],
    /// Compiled binary bytes.
    pub binary: &'a [u8],
    /// Pre-computed ABI layout hash bytes.
    pub abi_hash: &'a [u8],
    /// Optional DER-encoded PKCS#8 ECDSA-P256 private key for signing.
    pub signing_key: Option<&'a [u8]>,
    /// Isolation tier to certify into.
    pub isolation_tier: IsolationTier,
}

/// Generate a certification (Brand Seal) from raw inputs.
pub fn generate_certification(input: SealInput<'_>) -> CertResult<Certification> {
    // 1. Compute individual SHA-256 digests.
    let source_hash = sha256(input.source_ast);
    let audit_hash = sha256(input.audit_report);
    let binary_hash = sha256(input.binary);

    // 2. Concatenate: source ‖ audit ‖ 0x00 (exit code) ‖ binary.
    let mut concat = Vec::with_capacity(32 * 3 + 1);
    concat.extend_from_slice(&source_hash);
    concat.extend_from_slice(&audit_hash);
    concat.push(0x00); // successful exit status
    concat.extend_from_slice(&binary_hash);

    // 3. Final seal hash.
    let seal_hash = sha256(&concat);
    let seal_hex = hex_encode(&seal_hash);

    // 4. Optional ECDSA-P256 signature (signs the hex-encoded seal string).
    let signature = if let Some(key) = input.signing_key {
        let sig = sign_seal(seal_hex.as_bytes(), key)?;
        Some(base64::engine::general_purpose::STANDARD.encode(sig))
    } else {
        None
    };

    let abi_layout_hash = hex_encode(input.abi_hash);

    let created_at_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;

    Ok(Certification {
        seal: seal_hex,
        signature,
        abi_layout_hash,
        created_at_us,
        isolation_tier: input.isolation_tier,
    })
}

// ── Verification Engine ──────────────────────────────────────────────

/// Verify a stored certification against recomputed values.
///
/// Recomputes the seal from the provided source, audit, and binary data,
/// then compares it to the expected certification.  If a signing key was
/// used during generation, the signature is also verified.
pub fn verify_certification(
    stored: &Certification,
    source_ast: &[u8],
    audit_report: &[u8],
    binary: &[u8],
    abi_hash: &[u8],
    public_key: Option<&[u8]>,
) -> CertResult<()> {
    // 1. Recompute the seal.
    let source_h = sha256(source_ast);
    let audit_h = sha256(audit_report);
    let binary_h = sha256(binary);

    let mut concat = Vec::with_capacity(32 * 3 + 1);
    concat.extend_from_slice(&source_h);
    concat.extend_from_slice(&audit_h);
    concat.push(0x00);
    concat.extend_from_slice(&binary_h);

    let recomputed_seal = hex_encode(&sha256(&concat));

    // 2. Compare seal.
    if recomputed_seal != stored.seal {
        return Err(CertError::SealMismatch {
            expected: stored.seal.clone(),
            actual: recomputed_seal,
        });
    }

    // 3. Compare ABI layout hash.
    let recomputed_abi = hex_encode(abi_hash);
    if recomputed_abi != stored.abi_layout_hash {
        return Err(CertError::AbiMismatch {
            expected: stored.abi_layout_hash.clone(),
            actual: recomputed_abi,
        });
    }

    // 4. Verify signature if present (signature was over hex-encoded seal string).
    if let Some(sig_b64) = &stored.signature {
        let pub_key = public_key.ok_or_else(|| {
            CertError::SignatureError("signature present but no public key provided".into())
        })?;
        let sig_bytes = base64::engine::general_purpose::STANDARD
            .decode(sig_b64)
            .map_err(|e| CertError::SignatureError(e.to_string()))?;
        verify_signature(recomputed_seal.as_bytes(), &sig_bytes, pub_key)?;
    }

    Ok(())
}

// ── ABI Layout Hash Engine ───────────────────────────────────────────

/// A single struct fingerprint entry for ABI layout hashing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StructFingerprint {
    /// Fully qualified struct name.
    pub name: String,
    /// Ordered field descriptors: `name:type` pairs.
    pub fields: Vec<String>,
}

/// Compute the ABI layout hash for all public `#[repr(C)]` Pod structs
/// in a crate source tree.
///
/// Walks `crate_root/src/`, finds structs with `#[derive(Pod)]` and
/// `#[repr(C)]`, builds deterministic fingerprints, and returns the
/// SHA-256 of the concatenated sorted fingerprints.
pub fn compute_abi_layout_hash(crate_root: &Path) -> CertResult<Vec<u8>> {
    let src_dir = crate_root.join("src");
    if !src_dir.is_dir() {
        return Err(CertError::HashError(format!(
            "source directory not found: {}",
            src_dir.display()
        )));
    }

    let mut fingerprints = Vec::new();

    for entry in walkdir::WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        let src = std::fs::read_to_string(path)
            .map_err(|e| CertError::HashError(format!("failed to read {}: {}", path.display(), e)))?;

        let file = syn::parse_file(&src)
            .map_err(|e| CertError::HashError(format!("failed to parse {}: {}", path.display(), e)))?;

        for item in file.items {
            if let syn::Item::Struct(s) = item {
                let has_repr_c = s.attrs.iter().any(|a| {
                    if !a.path().is_ident("repr") {
                        return false;
                    }
                    match &a.meta {
                        syn::Meta::List(meta_list) => {
                            let tokens_str = meta_list.tokens.to_string();
                            tokens_str.contains('C') || tokens_str.contains("packed")
                        }
                        _ => false,
                    }
                });
                let derives_pod = s.attrs.iter().any(|a| {
                    if !a.path().is_ident("derive") {
                        return false;
                    }
                    match &a.meta {
                        syn::Meta::List(meta_list) => {
                            meta_list.tokens.to_string().contains("Pod")
                        }
                        _ => false,
                    }
                });

                if has_repr_c && derives_pod {
                    let mut fp = StructFingerprint {
                        name: s.ident.to_string(),
                        fields: Vec::new(),
                    };
                    for field in s.fields.iter() {
                        if let Some(ident) = &field.ident {
                            let ty = &field.ty;
                            let ty_str = quote::quote!(#ty).to_string();
                            fp.fields.push(format!("{}:{}", ident, ty_str));
                        }
                    }
                    fingerprints.push(fp);
                }
            }
        }
    }

    // Deterministic ordering.
    fingerprints.sort();

    // Concatenate fingerprints.
    let mut concatenated = String::new();
    for fp in &fingerprints {
        concatenated.push_str(&format!("struct:{};", fp.name));
        for field in &fp.fields {
            concatenated.push_str(field);
            concatenated.push(';');
        }
    }

    Ok(sha256(concatenated.as_bytes()).to_vec())
}

// ── Isolation Validator ──────────────────────────────────────────────

/// Validate that an ACC component meets isolation requirements for a
/// given tier.
pub fn validate_isolation(
    crate_root: &Path,
    tier: IsolationTier,
) -> CertResult<()> {
    // Walk source files and check for isolation violations.
    let src_dir = crate_root.join("src");
    if !src_dir.is_dir() {
        return Ok(()); // No source — nothing to violate.
    }

    for entry in walkdir::WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        let src = std::fs::read_to_string(path)
            .map_err(|e| CertError::HashError(format!("failed to read {}: {}", path.display(), e)))?;

        match tier {
            IsolationTier::Isolated => {
                // Isolated components must not use global mutable state.
                if src.contains("static mut ") {
                    return Err(CertError::IsolationViolation(format!(
                        "isolated component at {} uses `static mut`",
                        path.display()
                    )));
                }
                // Isolated components must not use thread-local storage.
                if src.contains("thread_local!") {
                    return Err(CertError::IsolationViolation(format!(
                        "isolated component at {} uses `thread_local!`",
                        path.display()
                    )));
                }
            }
            IsolationTier::Subordinate => {
                // Subordinate allows shared-nothing IPC but not raw pointers.
                if src.contains("*mut ") || src.contains("*const ") {
                    // Only flag raw pointers outside of unsafe blocks.
                    // A simple heuristic: flag any raw pointer usage.
                    return Err(CertError::IsolationViolation(format!(
                        "subordinate component at {} uses raw pointers",
                        path.display()
                    )));
                }
            }
            IsolationTier::Core => {
                // Core components have elevated privileges — minimal restrictions.
            }
        }
    }

    Ok(())
}

// ── Internal Helpers ─────────────────────────────────────────────────

/// SHA-256 wrapper returning raw 32-byte digest.
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Hex-encode a byte slice.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Sign a message using ECDSA-P256 with a DER-encoded PKCS#8 private key.
fn sign_seal(message: &[u8], pkcs8_der: &[u8]) -> CertResult<Vec<u8>> {
    let rng = ring::rand::SystemRandom::new();
    let key_pair = ring::signature::EcdsaKeyPair::from_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
        pkcs8_der,
        &rng,
    )
    .map_err(|e| CertError::SignatureError(format!("invalid signing key: {}", e)))?;

    let sig = key_pair
        .sign(&rng, message)
        .map_err(|e| CertError::SignatureError(format!("signing failed: {}", e)))?;

    Ok(sig.as_ref().to_vec())
}

/// Verify an ECDSA-P256 signature using a raw public key bytes.
fn verify_signature(
    message: &[u8],
    signature: &[u8],
    public_key_bytes: &[u8],
) -> CertResult<()> {
    let public_key = ring::signature::UnparsedPublicKey::new(
        &ring::signature::ECDSA_P256_SHA256_FIXED,
        public_key_bytes,
    );
    public_key
        .verify(message, signature)
        .map_err(|_| CertError::SignatureError("signature verification failed".into()))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::KeyPair;

    #[test]
    fn sha256_produces_deterministic_output() {
        let data = b"hello world";
        let h1 = sha256(data);
        let h2 = sha256(data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);
    }

    #[test]
    fn sha256_empty_input() {
        let h = sha256(b"");
        // SHA-256 of empty string is well-known.
        assert_eq!(
            hex_encode(&h),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hex_encode_roundtrip() {
        let bytes = [0x00, 0xff, 0xab, 0x12];
        let hex = hex_encode(&bytes);
        assert_eq!(hex, "00ffab12");
    }

    #[test]
    fn generate_and_verify_seal_no_signing() {
        let source = b"fn main() {}";
        let audit = b"{\"violations\":[]}";
        let binary = b"\x7fELF";
        let abi = b"abi_hash_data";

        let cert = generate_certification(SealInput {
            source_ast: source,
            audit_report: audit,
            binary,
            abi_hash: abi,
            signing_key: None,
            isolation_tier: IsolationTier::Isolated,
        })
        .unwrap();

        assert!(!cert.seal.is_empty());
        assert!(cert.signature.is_none());
        assert_eq!(cert.isolation_tier, IsolationTier::Isolated);

        // Verify should pass with matching inputs.
        verify_certification(&cert, source, audit, binary, abi, None).unwrap();
    }

    #[test]
    fn verify_rejects_tampered_source() {
        let source = b"fn main() {}";
        let audit = b"{\"violations\":[]}";
        let binary = b"\x7fELF";
        let abi = b"abi_hash_data";

        let cert = generate_certification(SealInput {
            source_ast: source,
            audit_report: audit,
            binary,
            abi_hash: abi,
            signing_key: None,
            isolation_tier: IsolationTier::Isolated,
        })
        .unwrap();

        // Tamper with source — verification should fail.
        let result = verify_certification(&cert, b"fn main() { /* tampered */ }", audit, binary, abi, None);
        assert!(result.is_err());
        match result.unwrap_err() {
            CertError::SealMismatch { .. } => {}
            other => panic!("expected SealMismatch, got: {:?}", other),
        }
    }

    #[test]
    fn verify_rejects_tampered_abi_hash() {
        let source = b"fn main() {}";
        let audit = b"{\"violations\":[]}";
        let binary = b"\x7fELF";
        let abi = b"abi_hash_data";

        let cert = generate_certification(SealInput {
            source_ast: source,
            audit_report: audit,
            binary,
            abi_hash: abi,
            signing_key: None,
            isolation_tier: IsolationTier::Isolated,
        })
        .unwrap();

        let result = verify_certification(&cert, source, audit, binary, b"wrong_abi", None);
        assert!(result.is_err());
        match result.unwrap_err() {
            CertError::AbiMismatch { .. } => {}
            other => panic!("expected AbiMismatch, got: {:?}", other),
        }
    }

    #[test]
    fn generate_seal_with_signing_key() {
        // Generate an ECDSA-P256 key pair for testing.
        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = ring::signature::EcdsaKeyPair::generate_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            &rng,
        )
        .unwrap();

        let source = b"test source";
        let audit = b"test audit";
        let binary = b"test binary";
        let abi = b"test abi";

        let cert = generate_certification(SealInput {
            source_ast: source,
            audit_report: audit,
            binary,
            abi_hash: abi,
            signing_key: Some(pkcs8.as_ref()),
            isolation_tier: IsolationTier::Subordinate,
        })
        .unwrap();

        assert!(cert.signature.is_some());
        assert_eq!(cert.isolation_tier, IsolationTier::Subordinate);

        // Extract public key from the key pair for verification.
        let key_pair = ring::signature::EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            pkcs8.as_ref(),
            &rng,
        )
        .unwrap();
        let pub_key = key_pair.public_key();

        // Verify should pass.
        verify_certification(&cert, source, audit, binary, abi, Some(pub_key.as_ref())).unwrap();
    }

    #[test]
    fn verify_rejects_tampered_signature() {
        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = ring::signature::EcdsaKeyPair::generate_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            &rng,
        )
        .unwrap();

        let key_pair = ring::signature::EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            pkcs8.as_ref(),
            &rng,
        )
        .unwrap();
        let pub_key = key_pair.public_key();

        let cert = generate_certification(SealInput {
            source_ast: b"src",
            audit_report: b"aud",
            binary: b"bin",
            abi_hash: b"abi",
            signing_key: Some(pkcs8.as_ref()),
            isolation_tier: IsolationTier::Isolated,
        })
        .unwrap();

        // Tamper with the signature.
        let mut bad_cert = cert.clone();
        bad_cert.signature = Some("invalidsignature==".into());

        let result = verify_certification(
            &bad_cert,
            b"src",
            b"aud",
            b"bin",
            b"abi",
            Some(pub_key.as_ref()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn verify_requires_public_key_when_signature_present() {
        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = ring::signature::EcdsaKeyPair::generate_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            &rng,
        )
        .unwrap();

        let cert = generate_certification(SealInput {
            source_ast: b"src",
            audit_report: b"aud",
            binary: b"bin",
            abi_hash: b"abi",
            signing_key: Some(pkcs8.as_ref()),
            isolation_tier: IsolationTier::Isolated,
        })
        .unwrap();

        // Verify without providing public key should fail.
        let result = verify_certification(&cert, b"src", b"aud", b"bin", b"abi", None);
        assert!(result.is_err());
    }

    #[test]
    fn isolation_tier_display() {
        assert_eq!(IsolationTier::Isolated.to_string(), "isolated");
        assert_eq!(IsolationTier::Subordinate.to_string(), "subordinate");
        assert_eq!(IsolationTier::Core.to_string(), "core");
    }

    #[test]
    fn validate_isolation_rejects_static_mut() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("lib.rs"),
            "static mut COUNTER: u64 = 0;\npub fn get() -> u64 { unsafe { COUNTER } }\n",
        )
        .unwrap();

        let result = validate_isolation(dir.path(), IsolationTier::Isolated);
        assert!(result.is_err());
        match result.unwrap_err() {
            CertError::IsolationViolation(msg) => {
                assert!(msg.contains("static mut"));
            }
            other => panic!("expected IsolationViolation, got: {:?}", other),
        }
    }

    #[test]
    fn validate_isolation_rejects_thread_local() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("lib.rs"),
            "thread_local! {{ static X: u32 = 0; }}\n",
        )
        .unwrap();

        let result = validate_isolation(dir.path(), IsolationTier::Isolated);
        assert!(result.is_err());
    }

    #[test]
    fn validate_isolation_core_allows_everything() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("lib.rs"),
            "static mut GLOBAL: u64 = 0;\n",
        )
        .unwrap();

        // Core tier should pass.
        validate_isolation(dir.path(), IsolationTier::Core).unwrap();
    }

    #[test]
    fn compute_abi_layout_hash_empty_crate() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("lib.rs"), "// empty\n").unwrap();

        let hash = compute_abi_layout_hash(dir.path()).unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn compute_abi_layout_hash_finds_pod_structs() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("lib.rs"),
            "#[repr(C)]\n#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]\n\
             pub struct Header {\n    pub tag: u32,\n    pub len: u32,\n}\n",
        )
        .unwrap();

        let hash = compute_abi_layout_hash(dir.path()).unwrap();
        assert_eq!(hash.len(), 32);

        // Verify determinism.
        let hash2 = compute_abi_layout_hash(dir.path()).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn compute_abi_layout_hash_ignores_non_pod_structs() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("lib.rs"),
            "pub struct Config { pub name: String }\n",
        )
        .unwrap();

        let hash = compute_abi_layout_hash(dir.path()).unwrap();
        // Should be hash of empty concatenation.
        let empty_hash = sha256(b"");
        assert_eq!(hash, empty_hash);
    }
}
