//! CLAUDE.md conventions check.
//!
//! Unlike the other angles, this one is only worth running an LLM call for
//! when there's something to check it against: a directory-scoped
//! CLAUDE.md/CLAUDE.local.md that actually governs one of the changed files.
//! Discovery is plain filesystem work; only the violation check itself goes
//! to the model, and only when a governing file exists.

use std::path::{Path, PathBuf};

use anyhow::Result;
use llm_gateway::LLMClient;
use walkdir::WalkDir;

use crate::angles::{Angle, Candidate};

/// Find every CLAUDE.md / CLAUDE.local.md under `repo` whose directory is an
/// ancestor of at least one of `changed_files` (paths relative to `repo`).
pub fn find_governing_files(repo: &Path, changed_files: &[String]) -> Vec<PathBuf> {
    let mut governing = Vec::new();
    for entry in WalkDir::new(repo).into_iter().filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy();
        if name != "CLAUDE.md" && name != "CLAUDE.local.md" {
            continue;
        }
        let dir = match entry.path().parent() {
            Some(d) => d,
            None => continue,
        };
        let governs_something = changed_files.iter().any(|f| {
            let full = repo.join(f);
            full.starts_with(dir)
        });
        if governs_something {
            governing.push(entry.path().to_path_buf());
        }
    }
    governing
}

const CONVENTIONS_SYSTEM_PROMPT: &str = "\
You check a diff against one or more CLAUDE.md convention files. Only flag a violation when \
you can quote BOTH the exact rule text (with its file path) AND the exact line in the diff \
that breaks it. No style preferences, no 'spirit of the doc' inferences — literal, quotable \
violations only. Respond with ONLY a JSON array (no prose) of up to 6 objects shaped as: \
{\"file\": \"path\", \"line\": <int or null>, \"summary\": \"rule quoted + what breaks it\", \
\"failure_scenario\": \"why this is a real violation, not a preference\"}. If nothing \
qualifies, respond with [].";

/// Run the conventions check. Returns an empty vec (no LLM call made) when
/// no CLAUDE.md/CLAUDE.local.md governs any changed file.
pub async fn run(
    client: &LLMClient,
    repo: &Path,
    changed_files: &[String],
    diff_text: &str,
) -> Result<Vec<Candidate>> {
    let governing = find_governing_files(repo, changed_files);
    if governing.is_empty() {
        return Ok(Vec::new());
    }

    let mut rules = String::new();
    for path in &governing {
        let rel = path.strip_prefix(repo).unwrap_or(path);
        rules.push_str(&format!("\n--- {} ---\n", rel.display()));
        rules.push_str(&std::fs::read_to_string(path).unwrap_or_default());
    }

    let user = format!(
        "Governing CLAUDE.md files:\n{rules}\n\nDiff under review:\n```diff\n{diff_text}\n```"
    );

    let response = client
        .chat(CONVENTIONS_SYSTEM_PROMPT, &user, "compliance_audit")
        .await?;

    let start = match response.find('[') {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };
    let end = match response.rfind(']') {
        Some(e) if e >= start => e,
        _ => return Ok(Vec::new()),
    };

    #[derive(serde::Deserialize)]
    struct Raw {
        file: String,
        line: Option<u32>,
        summary: String,
        failure_scenario: String,
    }

    let raw: Vec<Raw> = serde_json::from_str(&response[start..=end]).unwrap_or_default();
    Ok(raw
        .into_iter()
        .map(|r| Candidate {
            angle: Angle::Conventions,
            file: r.file,
            line: r.line,
            summary: r.summary,
            failure_scenario: r.failure_scenario,
        })
        .collect())
}
