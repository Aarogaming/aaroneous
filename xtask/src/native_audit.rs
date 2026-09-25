use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::process::Command;

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    links: Option<String>,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
}

#[derive(Deserialize)]
struct Policy {
    allowed: Vec<AllowedPackage>,
}

#[derive(Deserialize)]
struct AllowedPackage {
    name: String,
    version: String,
}

pub fn run() -> Result<()> {
    println!("=== Native Dependency Boundary Audit ===");

    // 1. Get Cargo metadata
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--filter-platform",
            "x86_64-pc-windows-msvc",
        ])
        .output()
        .context("Failed to run cargo metadata")?;

    if !output.status.success() {
        bail!("cargo metadata failed");
    }

    let meta: Metadata = serde_json::from_slice(&output.stdout)?;

    // 2. Load allowlist
    let policy_str = std::fs::read_to_string("native-policy.toml")
        .context("Failed to read native-policy.toml")?;
    let policy: Policy = toml::from_str(&policy_str)?;
    let mut allowed_map = HashMap::new();
    for pkg in policy.allowed {
        allowed_map.entry(pkg.name).or_insert_with(Vec::new).push(pkg.version);
    }

    // 3. Identify all packages with native provenance
    let native_indicators = ["cc", "bindgen", "libloading", "cmake", "clang-sys", "pkg-config", "vcpkg"];

    let mut violations = Vec::new();

    for pkg in &meta.packages {
        let mut is_native = false;
        let mut reasons = Vec::new();

        if let Some(links) = &pkg.links {
            is_native = true;
            reasons.push(format!("links=\"{}\"", links));
        }

        for dep in &pkg.dependencies {
            if native_indicators.contains(&dep.name.as_str()) {
                is_native = true;
                reasons.push(format!("depends on {}", dep.name));
            }
        }

        if is_native {
            let allowed_versions = allowed_map.get(&pkg.name);
            let mut is_allowed = false;
            if let Some(versions) = allowed_versions {
                if versions.contains(&pkg.version) {
                    is_allowed = true;
                }
            }

            if !is_allowed {
                violations.push(format!(
                    "Package {} v{} has undeclared native provenance: {}",
                    pkg.name,
                    pkg.version,
                    reasons.join(", ")
                ));
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("Found {} native boundary violations:", violations.len());
        for v in violations {
            eprintln!("  - {}", v);
        }
        bail!("Native Dependency Boundary Audit failed.");
    }

    println!("Native Dependency Boundary Audit passed.");
    Ok(())
}
