#![deny(unsafe_code)]

mod gate;
mod encoding;
mod install;
mod uninstall;
mod package;

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let subcommand = args.get(1).map(String::as_str).unwrap_or("help");
    match subcommand {
        "gate" => gate::run(),
        "check-encoding" => encoding::run(),
        "install" => install::run(&args[2..]),
        "uninstall" => uninstall::run(),
        "package" => package::run(&args[2..]),
        "help" | "--help" | "-h" => { print_help(); Ok(()) }
        other => bail!("Unknown xtask subcommand: '{}'. Run `cargo xtask help` for usage.", other),
    }
}

fn print_help() {
    println!(r#"
Usage: cargo xtask <subcommand>

Subcommands:
  gate             Run all 5 mandatory verification gates (replaces scripts/agent_check.sh)
  check-encoding   Validate UTF-8/LF compliance across all tracked files
  install          Install Aaroneous on Windows (copies binaries, sets PATH, creates shortcuts)
  uninstall        Remove Aaroneous from Windows
  package          Package a release ZIP distribution

Examples:
  cargo xtask gate
  cargo xtask install --dir C:\Programs\Aaroneous
  cargo xtask package --version 0.3.2
"#);
}
