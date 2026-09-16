use anyhow::{Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run() -> Result<()> {
    let output = Command::new("git").args(["ls-files"]).output()?;

    if !output.status.success() {
        bail!("git ls-files failed");
    }

    let files_str = std::str::from_utf8(&output.stdout)?;
    let mut violations = Vec::new();

    let binary_exts = [
        ".png",
        ".ico",
        ".wasm",
        ".dll",
        ".exe",
        ".bin",
        ".db",
        ".gguf",
        ".si",
        ".sovereign",
    ];

    for line in files_str.lines() {
        if line.is_empty() {
            continue;
        }
        let path = Path::new(line);
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            let ext_with_dot = format!(".{}", ext);
            if binary_exts.contains(&ext_with_dot.as_str()) {
                continue;
            }
        }

        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(_) => continue, // Ignore files that don't exist anymore or can't be read
        };

        let text = match std::str::from_utf8(&bytes) {
            Ok(s) => s,
            Err(_) => {
                violations.push(format!("{}: Not valid UTF-8", line));
                continue;
            }
        };

        let is_bat_or_cmd = path
            .extension()
            .map(|ext| ext == "bat" || ext == "cmd")
            .unwrap_or(false);

        let has_crlf = text.contains("\r\n");
        let has_lf = text.replace("\r\n", "").contains('\n'); // Has LF that is not CRLF

        if is_bat_or_cmd {
            if has_lf {
                violations.push(format!("{}: Expected CRLF, found LF without CR", line));
            }
        } else if path.extension().and_then(|s| s.to_str()) != Some("ps1") && has_crlf {
            violations.push(format!("{}: Expected LF, found CRLF", line));
        }
    }

    if !violations.is_empty() {
        println!("Encoding violations found:");
        for v in &violations {
            println!("  {}", v);
        }
        bail!("Found {} encoding violations", violations.len());
    }

    println!("All text files pass encoding checks.");
    Ok(())
}
