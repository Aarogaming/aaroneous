// dev/tools/afc/src/recipe/filter.rs
use regex::Regex;

#[derive(Debug, Clone)]
pub struct DiagnosticEntry {
    pub level: String,
    pub code: Option<String>,
    pub message: String,
    pub location: Option<String>,
}

pub struct DiagnosticsFilter;

impl DiagnosticsFilter {
    /// Extract structured compiler diagnostics from raw terminal output
    pub fn parse_diagnostics(raw_output: &str) -> Vec<DiagnosticEntry> {
        let error_re = Regex::new(r"(?m)^(error|warning)(?:\[(E\d+)\])?: (.*)").ok();
        let loc_re = Regex::new(r"(?m)^\s+-->\s+(.*:\d+:\d+)").ok();

        let mut entries = Vec::new();
        let Some(err_regex) = error_re else {
            return entries;
        };
        let loc_regex = loc_re;

        let lines: Vec<&str> = raw_output.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = err_regex.captures(line) {
                let level = caps.get(1).map_or("error", |m| m.as_str()).to_string();
                let code = caps.get(2).map(|m| m.as_str().to_string());
                let message = caps.get(3).map_or("", |m| m.as_str()).to_string();

                // Look ahead 1-3 lines for location
                let mut location = None;
                if let Some(ref l_reg) = loc_regex {
                    for lookahead in 1..=3 {
                        if let Some(next_line) = lines.get(i + lookahead) {
                            if let Some(l_caps) = l_reg.captures(next_line) {
                                if let Some(m) = l_caps.get(1) {
                                    location = Some(m.as_str().to_string());
                                    break;
                                }
                            }
                        }
                    }
                }

                entries.push(DiagnosticEntry {
                    level,
                    code,
                    message,
                    location,
                });
            }
        }

        entries
    }

    /// Produce a compact, clean error summary specifically formatted for LLM prompts without token bloat
    pub fn summarize_for_prompt(raw_output: &str, max_entries: usize) -> String {
        let diagnostics = Self::parse_diagnostics(raw_output);
        if diagnostics.is_empty() {
            // Fallback: take last 10 non-empty lines if no regex match
            let lines: Vec<&str> = raw_output
                .lines()
                .filter(|l| !l.trim().is_empty())
                .collect();
            let start = lines.len().saturating_sub(10);
            return lines[start..].join("\n");
        }

        let mut summary = String::new();
        let count = diagnostics.len().min(max_entries);
        for entry in &diagnostics[..count] {
            let code_str = entry
                .code
                .as_deref()
                .map_or(String::new(), |c| format!("[{c}]"));
            let loc_str = entry
                .location
                .as_deref()
                .map_or(String::new(), |l| format!(" at {l}"));
            summary.push_str(&format!(
                "{}:{code_str} {}{loc_str}\n",
                entry.level.to_uppercase(),
                entry.message
            ));
        }

        if diagnostics.len() > max_entries {
            summary.push_str(&format!(
                "... and {} additional diagnostic(s) truncated.\n",
                diagnostics.len() - max_entries
            ));
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_filtering() {
        let raw = r#"
   Compiling sample v0.1.0 (D:\sample)
error[E0382]: borrow of moved value: `foo`
  --> src/main.rs:12:5
   |
11 |     let foo = String::new();
12 |     drop(foo);
   |
warning: unused variable: `bar`
  --> src/main.rs:15:9
   |
15 |     let bar = 42;
   |         ^^^
"#;
        let entries = DiagnosticsFilter::parse_diagnostics(raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].level, "error");
        assert_eq!(entries[0].code.as_deref(), Some("E0382"));
        assert_eq!(entries[0].location.as_deref(), Some("src/main.rs:12:5"));

        assert_eq!(entries[1].level, "warning");
        assert_eq!(entries[1].code, None);
        assert_eq!(entries[1].location.as_deref(), Some("src/main.rs:15:9"));

        let summary = DiagnosticsFilter::summarize_for_prompt(raw, 1);
        assert!(summary.contains("ERROR:[E0382]"));
        assert!(summary.contains("src/main.rs:12:5"));
        assert!(summary.contains("additional diagnostic(s) truncated"));
    }
}
