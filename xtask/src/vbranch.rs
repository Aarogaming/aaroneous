use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub use crate::base36::{
    LINEAGE_DIGEST_BYTES, LINEAGE_ID_LEN, MAX_ENCODED_CHARS, MAX_INPUT_BYTES, decode_base36,
    encode_base36, lineage_id,
};

pub fn append_to_log(log_path: &Path, lineage: &str, message: &str) -> Result<()> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let source_hash = hex::encode(Sha256::digest(message.as_bytes()));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(file, "LINEAGE: {lineage} | SOURCE_SHA256: {source_hash}")?;
    Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
    if args.is_empty() {
        bail!("Usage: cargo xtask vbranch <encode|decode|stamp|log> <payload>");
    }

    match args[0].as_str() {
        "encode" => {
            if args.len() < 2 {
                bail!("Usage: cargo xtask vbranch encode <message>");
            }
            let message = &args[1];
            let encoded = encode_base36(message.as_bytes())?;
            println!("{}", encoded);
        }
        "decode" => {
            if args.len() < 2 {
                bail!("Usage: cargo xtask vbranch decode <base36_payload>");
            }
            let payload = &args[1];
            let decoded = decode_base36(payload)?;
            println!(
                "{}",
                String::from_utf8(decoded).context("decoded payload is not UTF-8")?
            );
        }
        "stamp" => {
            if args.len() < 3 {
                bail!("Usage: cargo xtask vbranch stamp <manifest-path> <message>");
            }
            let cargo_toml_path = Path::new(&args[1]);
            let message = &args[2];
            let encoded = lineage_id(message)?;
            let root = cargo_toml_path
                .parent()
                .context("manifest path must have a parent directory")?;
            append_to_log(&root.join(".vbranch").join("HEAD"), &encoded, message)?;

            let content = std::fs::read_to_string(cargo_toml_path)
                .with_context(|| format!("failed to read {}", cargo_toml_path.display()))?;

            // We'll use regex to update the workspace version.
            let re = regex::Regex::new(
                r#"^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?)(?:\+[0-9A-Za-z.-]+)?""#,
            )?;
            let mut found = false;
            let mut new_content = String::new();
            let mut in_workspace_package = false;

            for line in content.lines() {
                if line == "[workspace.package]" {
                    in_workspace_package = true;
                } else if in_workspace_package && line.starts_with('[') {
                    in_workspace_package = false;
                }
                if in_workspace_package
                    && !found
                    && line.starts_with("version = ")
                    && let Some(caps) = re.captures(line)
                {
                    let base_version = &caps[1];
                    // Using build metadata (+) for the tracking payload
                    new_content
                        .push_str(&format!("version = \"{}+vb.{}\"\n", base_version, encoded));
                    found = true;
                    continue;
                }
                new_content.push_str(line);
                new_content.push('\n');
            }

            if !found {
                bail!("Could not find workspace version in Cargo.toml");
            }

            std::fs::write(cargo_toml_path, new_content)
                .with_context(|| format!("failed to write {}", cargo_toml_path.display()))?;
            println!(
                "Stamped {} with version payload: vb.{}",
                cargo_toml_path.display(),
                encoded
            );
        }
        "log" => {
            let log_path = args
                .get(1)
                .context("Usage: cargo xtask vbranch log <log-path>")?;
            if Path::new(log_path).exists() {
                let content = std::fs::read_to_string(log_path)?;
                print!("{}", content);
            } else {
                println!("No vbranch history found.");
            }
        }
        _ => bail!("Unknown vbranch command. Use 'encode', 'decode', 'stamp', or 'log'."),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base36_round_trip_preserves_leading_zeroes() {
        let source = b"\0\0Aaroneous";
        let encoded = encode_base36(source).unwrap();
        assert_eq!(decode_base36(&encoded).unwrap(), source);
    }

    #[test]
    fn invalid_base36_is_an_error_not_a_panic() {
        assert!(decode_base36("abc!").is_err());
    }

    #[test]
    fn lineage_is_stable_lowercase_and_fixed_width() {
        let first = lineage_id("architecture/refactor").unwrap();
        let second = lineage_id("architecture/refactor").unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), LINEAGE_ID_LEN);
        assert!(
            first
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        );
    }
}
