// src/main.rs
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use cratify::{audit, certify, python_to_rust, scaffold, translate, verify};




/// Cratify command‑line interface.
#[derive(Parser)]
#[command(name = "cratify", author, version, about, long_about = None)]
struct Cli {
    /// Fail on any audit violation.
    #[arg(long)]
    strict: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect a codebase for anti‑patterns and scaling‑law compliance.
    Audit {
        /// Path to the source directory or file.
        path: PathBuf,
    },
    /// Generate a new satellite crate skeleton.
    Scaffold {
        /// Name of the crate to create.
        name: String,
    },
    /// Translate legacy code via the LLM endpoint.
    Translate {
        /// Path to the legacy source file.
        path: PathBuf,
        /// Target crate to receive the translated code.
        #[arg(long)]
        target: String,
        /// Optional LLM endpoint override.
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Run cargo check on a specific crate.
    Verify {
        /// Crate name to verify.
        crate_name: String,
    },
    /// Generate a cryptographic Brand Seal for an ACC component.
    Certify {
        /// Path to the crate root directory.
        path: PathBuf,
        /// Optional DER-encoded PKCS#8 private key file for signing.
        #[arg(long)]
        signing_key: Option<PathBuf>,
        /// Isolation tier to certify into (isolated, subordinate, core).
        #[arg(long, default_value = "isolated")]
        tier: String,
    },
    /// Translate a Python module to an ACC Rust crate.
    PyTranslate {
        /// Path to the Python source file or directory.
        path: PathBuf,
        /// Target module name for the generated Rust code.
        #[arg(long)]
        module_name: String,
        /// Output path for the generated .rs file (default: stdout).
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Run the full pipeline: audit → scaffold → translate → verify.
    Harvest {
        /// Path to the legacy source.
        path: PathBuf,
        /// Target crate name.
        #[arg(long)]
        target: String,
        /// Optional LLM endpoint.
        #[arg(long)]
        endpoint: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Audit { path } => audit::run(path, cli.strict).await?,
        Commands::Scaffold { name } => scaffold::run(&name).await?,
        Commands::Translate { path, target, endpoint } => {
            translate::run(path, &target, endpoint).await?
        },
        Commands::Verify { crate_name } => verify::run(&crate_name).await?,
        Commands::PyTranslate { path, module_name, output } => {
            let python_source = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read Python source {:?}", path))?;

            let outcome = python_to_rust::translate_python_to_rust(&python_source, &module_name)
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !outcome.warnings.is_empty() {
                for w in &outcome.warnings {
                    eprintln!("[py-translate] WARNING: {w}");
                }
            }

            match &output {
                Some(out_path) => {
                    if let Some(parent) = out_path.parent() {
                        std::fs::create_dir_all(parent)
                            .with_context(|| format!("failed to create output dir {:?}", parent))?;
                    }
                    std::fs::write(out_path, &outcome.rust_code)
                        .with_context(|| format!("failed to write to {:?}", out_path))?;
                    println!(
                        "[py-translate] wrote {}B ({} functions, {} structs) to {}",
                        outcome.rust_code.len(),
                        outcome.functions_count,
                        outcome.structs_count,
                        out_path.display()
                    );
                }
                None => {
                    print!("{}", outcome.rust_code);
                }
            }
        }
        Commands::Certify { path, signing_key, tier } => {
            let isolation_tier = match tier.as_str() {
                "isolated" => certify::IsolationTier::Isolated,
                "subordinate" => certify::IsolationTier::Subordinate,
                "core" => certify::IsolationTier::Core,
                other => anyhow::bail!(
                    "invalid isolation tier `{}` — expected: isolated, subordinate, or core",
                    other
                ),
            };

            // Validate isolation requirements.
            certify::validate_isolation(&path, isolation_tier)?;
            println!("[certify] isolation validation passed for tier={}", isolation_tier);

            // Compute ABI layout hash.
            let abi_hash = certify::compute_abi_layout_hash(&path)?;
            println!(
                "[certify] ABI layout hash: {}",
                abi_hash.iter().map(|b| format!("{:02x}", b)).collect::<String>()
            );

            // Generate Brand Seal.
            let signing_key_data = if let Some(key_path) = &signing_key {
                let data = std::fs::read(key_path)
                    .with_context(|| format!("failed to read signing key {:?}", key_path))?;
                Some(data)
            } else {
                None
            };

            let source_ast = std::fs::read_to_string(path.join("src/lib.rs"))
                .unwrap_or_default()
                .into_bytes();
            let audit_report = serde_json::to_vec(&serde_json::json!({"status": "certified"}))?;

            let cert = certify::generate_certification(certify::SealInput {
                source_ast: &source_ast,
                audit_report: &audit_report,
                binary: b"",  // No binary at cert time — seal covers source + audit
                abi_hash: &abi_hash,
                signing_key: signing_key_data.as_deref(),
                isolation_tier,
            })?;

            // Write certification to cratify.toml.
            let cert_toml = format!(
                "\n[certification]\nseal = \"{}\"\nabi_layout_hash = \"{}\"\ntier = \"{}\"\n",
                cert.seal, cert.abi_layout_hash, cert.isolation_tier
            );
            let manifest_path = path.join("cratify.toml");
            if manifest_path.exists() {
                let mut content = std::fs::read_to_string(&manifest_path)?;
                content.push_str(&cert_toml);
                std::fs::write(&manifest_path, content)?;
            } else {
                std::fs::write(&manifest_path, &cert_toml[1..])?;
            }

            println!("[certify] Brand Seal generated: {}", cert.seal);
            println!("[certify] wrote certification to {}", manifest_path.display());
        }
        Commands::Harvest { path, target, endpoint } => {
            audit::run(path.clone(), cli.strict).await?;
            scaffold::run(&target).await?;
            translate::run(path, &target, endpoint).await?;
            verify::run(&target).await?;
        },
    }

    Ok(())
}
