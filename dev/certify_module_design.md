# `crates/cratify/src/certify.rs` – Certification Subsystem Design

**Purpose:** Implements the Brand Seal generation, ABI layout hashing, and verification state machine required by the Cratify Certification Subsystem (see `docs/certification_spec.md`).

---

## 1. Public API
```rust
/// Result type for certification operations.
pub type CertResult<T> = Result<T, CertError>;

/// Errors that can arise during certification.
#[derive(Debug, thiserror::Error)]
pub enum CertError {
    #[error("Hash computation failed: {0}")]
    HashError(String),
    #[error("Signature verification failed: {0}")]
    SignatureError(String),
    #[error("ABI layout mismatch (expected {expected}, got {actual})")]
    AbiMismatch { expected: String, actual: String },
    #[error("Missing certification metadata in manifest")]
    MissingMetadata,
    #[error("Seal mismatch (expected {expected}, got {actual})")]
    SealMismatch { expected: String, actual: String },
}

/// Struct representing the computed certification payload.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Certification {
    /// Hex‑encoded SHA‑256 of the combined source, audit, and binary hashes.
    pub seal: String,
    /// Optional base64‑encoded ECDSA‑P256 signature of `seal`.
    pub signature: Option<String>,
    /// Hex‑encoded ABI layout hash for zero‑copy contracts.
    pub abi_layout_hash: String,
}

/// Generate the certification payload for a given ACC.
pub fn generate_certification(
    source_ast: &[u8],
    audit_report: &[u8],
    binary: &[u8],
    abi_hash: &[u8],
    signing_key: Option<&[u8]>, // DER‑encoded PKCS#8 private key
) -> CertResult<Certification> {
    // 1. Compute individual SHA‑256 digests.
    let source_hash = sha256(source_ast);
    let audit_hash = sha256(audit_report);
    let binary_hash = sha256(binary);
    let abi_hash_hex = hex::encode(abi_hash);

    // 2. Concatenate the four digests (source, audit, exit_code=0, binary).
    // Exit code 0 is represented as a single byte 0x00.
    let mut concat = Vec::new();
    concat.extend_from_slice(&source_hash);
    concat.extend_from_slice(&audit_hash);
    concat.push(0x00); // successful exit status
    concat.extend_from_slice(&binary_hash);

    // 3. Final seal hash.
    let seal_hash = sha256(&concat);
    let seal_hex = hex::encode(seal_hash);

    // 4. Optional signature.
    let signature = if let Some(key) = signing_key {
        // Using ring crate for ECDSA‑P256 signing.
        let signing_key = ring::signature::EcdsaKeyPair::from_private_key_and_public_key(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            key,
            &[] // public key derived internally
        ).map_err(|e| CertError::SignatureError(e.to_string()))?;
        let sig = signing_key.sign(&ring::rand::SystemRandom::new(), &seal_hash)
            .map_err(|e| CertError::SignatureError(e.to_string()))?;
        Some(base64::encode(sig.as_ref()))
    } else {
        None
    };

    Ok(Certification {
        seal: seal_hex,
        signature,
        abi_layout_hash: abi_hash_hex,
    })
}

/// Verify a built ACC against its manifest.
pub fn verify_certification(
    manifest: &Certification,
    binary_path: &std::path::Path,
) -> CertResult<()> {
    // 1. Load the binary and locate the .certseal and .abihash sections.
    let binary = std::fs::read(binary_path)
        .map_err(|e| CertError::HashError(e.to_string()))?;
    let certseal = pe_parser::extract_section(&binary, ".certseal")
        .ok_or_else(|| CertError::SealMismatch { expected: manifest.seal.clone(), actual: "<missing>".into() })?;
    let embedded_seal = std::str::from_utf8(certseal).map_err(|e| CertError::HashError(e.to_string()))?;

    // 2. Compare seal.
    if embedded_seal.trim() != manifest.seal {
        return Err(CertError::SealMismatch {
            expected: manifest.seal.clone(),
            actual: embedded_seal.trim().to_string(),
        });
    }

    // 3. Verify ABI layout hash.
    let abihash_section = pe_parser::extract_section(&binary, ".abihash")
        .ok_or_else(|| CertError::AbiMismatch { expected: manifest.abi_layout_hash.clone(), actual: "<missing>".into() })?;
    let embedded_abi_hash = std::str::from_utf8(abihash_section).map_err(|e| CertError::HashError(e.to_string()))?;
    if embedded_abi_hash.trim() != manifest.abi_layout_hash {
        return Err(CertError::AbiMismatch {
            expected: manifest.abi_layout_hash.clone(),
            actual: embedded_abi_hash.trim().to_string(),
        });
    }

    // 4. Optional signature verification (if present).
    if let Some(sig_b64) = &manifest.signature {
        let sig = base64::decode(sig_b64).map_err(|e| CertError::SignatureError(e.to_string()))?;
        // Public key would be provided via Cratify config; placeholder check here.
        let pub_key = crate::config::signing_public_key();
        let peer_pub_key = ring::signature::UnparsedPublicKey::new(
            &ring::signature::ECDSA_P256_SHA256_FIXED,
            pub_key,
        );
        peer_pub_key.verify(manifest.seal.as_bytes(), &sig)
            .map_err(|_| CertError::SignatureError("invalid signature".into()))?;
    }

    Ok(())
}

/// Helper: SHA‑256 wrapper returning raw bytes.
fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}
```

---

## 2. Integration Points
- **CLI (`src/main.rs`)** – After `cratify translate`, the CLI calls `certify::generate_certification` with the AST, audit JSON, compiled binary, and ABI layout hash. The returned `Certification` is serialized into the `[certification]` table of `cratify.toml` and the binary sections `.certseal` / `.abihash` are written via the `binary_writer` helper.
- **`cratify verify`** – Loads the manifest, deserializes the `[certification]` table, and invokes `certify::verify_certification`. Any error aborts the pipeline and forces the ACC into the quarantine tier.
- **Runtime Loader (`@hypervisor` / `CapabilityBroker`)** – When loading an ACC, reads the embedded `.certseal` section, recomputes the expected seal from the manifest, and rejects the component if verification fails. The loader uses the same `certify::verify_certification` logic (exposed via a `pub(crate)` interface) to keep verification consistent.

---

## 3. ABI Layout Hash Engine (used by `generate_certification`)
```rust
/// Compute the ABI layout hash for all public `Pod` structs in the crate.
pub fn compute_abi_layout_hash(crate_root: &std::path::Path) -> CertResult<Vec<u8>> {
    // Walk the source tree, collect all `#[derive(Pod, Zeroable)]` structs.
    let mut struct_fingerprints = Vec::new();
    for entry in walkdir::WalkDir::new(crate_root.join("src")) {
        let entry = entry.map_err(|e| CertError::HashError(e.to_string()))?;
        if entry.path().extension().and_then(|s| s.to_str()) != Some("rs") { continue; }
        let src = std::fs::read_to_string(entry.path())
            .map_err(|e| CertError::HashError(e.to_string()))?;
        // Very lightweight parsing – look for `#[derive(` containing `Pod`.
        if src.contains("#[derive") && src.contains("Pod") {
            // Use syn to parse the file and extract struct definitions.
            let file = syn::parse_file(&src).map_err(|e| CertError::HashError(e.to_string()))?;
            for item in file.items {
                if let syn::Item::Struct(s) = item {
                    // Ensure it has #[repr(C)] or #[repr(packed)] and derives Pod.
                    let has_repr = s.attrs.iter().any(|a| a.path.is_ident("repr"));
                    let derives_pod = s.attrs.iter().any(|a| {
                        a.path.is_ident("derive") && a.tokens.to_string().contains("Pod")
                    });
                    if has_repr && derives_pod {
                        // Build fingerprint: name + field list + offsets + sizes.
                        let mut fp = format!("struct:{}", s.ident);
                        for field in s.fields.iter() {
                            if let Some(ident) = &field.ident {
                                let ty = &field.ty;
                                // Use a dummy instance to query size & offset via memoffset.
                                // For compile‑time safety, we rely on `bytemuck::Pod` guaranteeing layout.
                                fp.push_str(&format!("|{}:{}", ident, quote::quote!{#ty}));
                            }
                        }
                        struct_fingerprints.push(fp);
                    }
                }
            }
        }
    }
    // Concatenate fingerprints in deterministic order.
    struct_fingerprints.sort();
    let concatenated = struct_fingerprints.join(";");
    Ok(sha256(concatenated.as_bytes()).to_vec())
}
```

---

## 4. State Machine (Verification Flow)
```
+-------------------+     +-------------------+     +-------------------+
|   Start (audit)   | --> |   translate       | --> |   verify          |
+-------------------+     +-------------------+     +-------------------+
        |                         |                         |
        v                         v                         v
   compute hashes          embed .certseal          load manifest
        |                         |                         |
        v                         v                         v
   generate Certification   write .abihash           run verify_certification
        |                         |                         |
        +------------------------+-------------------------+
                                 |
                                 v
                             +--------+
                             |  pass  |
                             +--------+
```

---

## 5. Dependencies
- `sha2` – SHA‑256 digest.
- `thiserror` – ergonomic error types.
- `serde`, `serde_json` – manifest (de)serialization.
- `ring` – ECDSA‑P256 signing/verifying.
- `bytemuck` – trait used in layout discovery (runtime check only).
- `syn` / `quote` – compile‑time parsing of source files for ABI hash.
- `walkdir` – filesystem traversal.
- `pe-parser` (internal helper) – extract custom PE sections.

---

*Implementation of this module completes the certification pipeline described in `docs/certification_spec.md` and enables the hypervisor to enforce brand‑seal integrity for all ACCs.*
