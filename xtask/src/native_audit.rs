use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::process::Command;

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    resolve: Resolve,
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
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    id: String,
    deps: Vec<NodeDep>,
}

#[derive(Deserialize)]
struct NodeDep {
    pkg: String,
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

    let mut pkg_map = HashMap::new();
    for pkg in &meta.packages {
        pkg_map.insert(pkg.id.clone(), pkg);
    }

    // 2. Identify all packages with native provenance
    let mut native_packages = Vec::new();

    let native_indicators = [
        "cc",
        "bindgen",
        "libloading",
        "cmake",
        "clang-sys",
        "pkg-config",
        "vcpkg",
    ];

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
            native_packages.push((pkg, reasons));
        }
    }

    // 3. Load allowlist
    // We will hardcode the temporary allowlist here based on C68
    // In the future, this should be parsed from a config file.
    let allowed_native = vec![
        "aws-lc-sys",
        "aws-lc-rs", // Uses links="aws_lc_rs_1_18_1_sys"
        "ring",
        "rusqlite",
        "libsqlite3-sys",
        "onig_sys",
        "zstd-sys",
        "ash",
        "windows",
        "windows-sys",
        "lz4-sys",
        "bzip2-sys",
        "libz-sys",
        "esaxx-rs",
        "windows-targets",
        "windows_x86_64_msvc",
        "windows_aarch64_msvc",
        "windows_i686_msvc",
        "windows_x86_64_gnullvm",
        "windows_aarch64_gnullvm",
        "windows_i686_gnu",
        "windows_x86_64_gnu",
        "windows-link",
        "winapi",
        "core-foundation-sys",
        "core-graphics-types",
        "core-text",
        "security-framework-sys",
        "wgpu-hal",
        "khronos-egl",
        "onig",
        "alloca",          // depends on cc
        "cmake",           // depends on cc
        "defmt",           // uses links without native
        "prettyplease",    // uses links without native
        "rayon-core",      // uses links without native
        "libmimalloc-sys", // P1: explicitly allowlisted for baseline, needs adapter
        "glutin",          // platform graphics API dependency
        "rfc0006_host",    // P1: development host loader
        "sdk",             // P1: production loader requires re-review
        "libloading",      // P1: underlying loading library
    ];

    let mut violations = Vec::new();

    for (pkg, reasons) in &native_packages {
        if !allowed_native.contains(&pkg.name.as_str()) {
            violations.push(format!(
                "Package {} v{} has undeclared native provenance: {}",
                pkg.name,
                pkg.version,
                reasons.join(", ")
            ));
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
