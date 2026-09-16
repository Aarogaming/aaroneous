#![deny(unsafe_code)]

mod encoding;
mod export_wit;
mod gate;
mod install;
mod package;
mod uninstall;

use anyhow::{Result, bail};
use std::process::Command;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let subcommand = args.get(1).map(String::as_str).unwrap_or("help");
    match subcommand {
        "gate" => gate::run(),
        "check-encoding" => encoding::run(),
        "install" => install::run(&args[2..]),
        "uninstall" => uninstall::run(),
        "package" => package::run(&args[2..]),
        "export-wit" => {
            let config = export_wit::WitExportConfig::default();
            export_wit::export_wit(&config).map_err(|e| anyhow::anyhow!(e))
        }
        "queue" => {
            println!("=== Launching Local Agent Passive Progress Daemon ===");
            let status = Command::new("pwsh")
                .args(["-File", "scripts/local_agent_daemon.ps1"])
                .status()?;
            if !status.success() {
                bail!("Local agent daemon failed with status: {}", status);
            }
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => bail!(
            "Unknown xtask subcommand: '{}'. Run `cargo xtask help` for usage.",
            other
        ),
    }
}

fn print_help() {
    println!(
        r#"
Usage: cargo xtask <subcommand>

Subcommands:
  gate             Run every CI verification gate locally (replaces scripts/agent_check.sh)
  check-encoding   Validate UTF-8/LF compliance across all tracked files
  export-wit       Export WebAssembly Interface Type (WIT) declarations from capabilities
  queue            Run the passive local agent background progress engine (Ollama GPU)
  install          Install Aaroneous on Windows (copies binaries, sets PATH, creates shortcuts)
  uninstall        Remove Aaroneous from Windows
  package          Package a release ZIP distribution

Examples:
  cargo xtask gate
  cargo xtask export-wit
  cargo xtask queue
  cargo xtask install --dir C:\Programs\Aaroneous
  cargo xtask package --version 0.3.2
"#
    );
}
