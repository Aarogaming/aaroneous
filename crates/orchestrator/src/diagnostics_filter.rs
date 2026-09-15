//! Diagnostics filter module for parsing Rust compiler output

use regex::Regex;

/// Represents a parsed Rust compiler diagnostic entry
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticEntry {
    pub level: String,
    pub code: Option<String>,
    pub message: String,
    pub location: Option<String>,
}

/// Filter for parsing and summarizing Rust compiler diagnostics
pub struct DiagnosticsFilter;

impl DiagnosticsFilter {
    /// Parse Rust compiler diagnostics from raw output
    ///
    /// Expected format:
    /// error[E0012]: ...
    /// --> src/main.rs:10:5
    ///    |
    ///    | help: ...
    ///
    /// warning[E0012]: ...
    /// --> src/main.rs:10:5
    ///    |
    ///    | help: ...
    pub fn parse_diagnostics(raw_output: &str) -> Vec<DiagnosticEntry> {
        let mut entries = Vec::new();

        // Regex to match error/warning lines with optional code
        let Ok(diag_regex) = Regex::new(r"(error|warning)(?:\[(E\d+)\])?:\s*(.*)") else {
            return entries;
        };

        // Regex to match location lines
        let Ok(location_regex) = Regex::new(r"^\s*-->\s*(.+):(\d+):(\d+)") else {
            return entries;
        };

        let mut current_entry: Option<DiagnosticEntry> = None;

        for line in raw_output.lines() {
            if let Some(caps) = diag_regex.captures(line) {
                if let Some(finished) = current_entry.take() {
                    entries.push(finished);
                }
                let level = caps.get(1).map_or("error", |m| m.as_str()).to_string();
                let code = caps.get(2).map(|m| m.as_str().to_string());
                let message = caps.get(3).map_or("", |m| m.as_str()).to_string();

                current_entry = Some(DiagnosticEntry {
                    level,
                    code,
                    message,
                    location: None,
                });
            } else if let Some(caps) = location_regex.captures(line)
                && let Some(ref mut entry) = current_entry
                && let (Some(f), Some(l), Some(c)) = (caps.get(1), caps.get(2), caps.get(3))
            {
                entry.location = Some(format!("{}:{}:{}", f.as_str(), l.as_str(), c.as_str()));
            }
        }

        if let Some(finished) = current_entry {
            entries.push(finished);
        }
        entries
    }

    /// Summarize diagnostics for a prompt, limiting to max_entries
    pub fn summarize_for_prompt(raw_output: &str, max_entries: usize) -> String {
        let entries = Self::parse_diagnostics(raw_output);

        if entries.is_empty() {
            return String::from("No diagnostics found.");
        }

        let mut summary = String::new();
        summary.push_str(&format!(
            "Found {} diagnostic{}:\n",
            entries.len(),
            if entries.len() == 1 { "" } else { "s" }
        ));

        let display_entries = entries.iter().take(max_entries).collect::<Vec<_>>();

        for (i, entry) in display_entries.iter().enumerate() {
            let prefix = if i < max_entries - 1 {
                "  "
            } else {
                "  ... (truncated)"
            };
            summary.push_str(&format!("{}{}: {}\n", prefix, entry.level, entry.message));

            if let Some(ref code) = entry.code {
                summary.push_str(&format!("    Code: {}\n", code));
            }

            if let Some(ref loc) = entry.location {
                summary.push_str(&format!("    Location: {}\n", loc));
            }
        }

        if entries.len() > max_entries {
            summary.push_str(&format!(
                "\n... and {} more diagnostics\n",
                entries.len() - max_entries
            ));
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_error() {
        let output = r#"error[E0012]: cannot find value `x` in this scope
   --> src/main.rs:10:5
    |
10  |     println!("{}", x);
    |                  ^ not found in this scope
    = note: `x` is not in scope
"#;

        let entries = DiagnosticsFilter::parse_diagnostics(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, "error");
        assert_eq!(entries[0].code, Some("E0012".to_string()));
        assert_eq!(entries[0].message, "cannot find value `x` in this scope");
        assert!(entries[0].location.is_some());
    }

    #[test]
    fn test_parse_single_warning() {
        let output = r#"warning[E0012]: unused variable: `y`
   --> src/main.rs:15:10
    |
15  |     let y = 42;
    |          ^ help: consider removing this binding
"#;

        let entries = DiagnosticsFilter::parse_diagnostics(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, "warning");
        assert_eq!(entries[0].code, Some("E0012".to_string()));
        assert_eq!(entries[0].message, "unused variable: `y`");
        assert!(entries[0].location.is_some());
    }

    #[test]
    fn test_parse_multiple_diagnostics() {
        let output = r#"error[E0012]: cannot find value `x` in this scope
   --> src/main.rs:10:5
    |
10  |     println!("{}", x);
    |                  ^ not found in this scope

warning[E0012]: unused variable: `y`
   --> src/main.rs:15:10
    |
15  |     let y = 42;
    |          ^ help: consider removing this binding

error[E0013]: type mismatch
   --> src/main.rs:20:8
    |
20  |     let z: i32 = "hello";
    |          ^^^^^^^^^^^^^^^^ expected i32, found &str
"#;

        let entries = DiagnosticsFilter::parse_diagnostics(output);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].level, "error");
        assert_eq!(entries[1].level, "warning");
        assert_eq!(entries[2].level, "error");
    }

    #[test]
    fn test_parse_empty_output() {
        let output = "";
        let entries = DiagnosticsFilter::parse_diagnostics(output);
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_parse_no_diagnostics() {
        let output = r#"Compiling project...
Finished in 0.5s
"#;
        let entries = DiagnosticsFilter::parse_diagnostics(output);
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_summarize_for_prompt() {
        let output = r#"error[E0012]: cannot find value `x` in this scope
   --> src/main.rs:10:5
    |
10  |     println!("{}", x);
    |                  ^ not found in this scope

warning[E0012]: unused variable: `y`
   --> src/main.rs:15:10
    |
15  |     let y = 42;
    |          ^ help: consider removing this binding
"#;

        let summary = DiagnosticsFilter::summarize_for_prompt(output, 2);
        assert!(summary.contains("Found 2 diagnostics"));
        assert!(summary.contains("error"));
        assert!(summary.contains("warning"));
    }

    #[test]
    fn test_summarize_for_prompt_truncated() {
        let output = r#"error[E0012]: cannot find value `x` in this scope
   --> src/main.rs:10:5
    |
10  |     println!("{}", x);
    |                  ^ not found in this scope

error[E0013]: type mismatch
   --> src/main.rs:20:8
    |
20  |     let z: i32 = "hello";
    |          ^^^^^^^^^^^^^^^^ expected i32, found &str

error[E0014]: another error
   --> src/main.rs:30:12
    |
30  |     let a = 1;
    |          ^ another error

error[E0015]: yet another error
   --> src/main.rs:40:16
    |
40  |     let b = 2;
    |          ^ yet another error
"#;

        let summary = DiagnosticsFilter::summarize_for_prompt(output, 2);
        assert!(summary.contains("Found 4 diagnostics"));
        assert!(summary.contains("... and 2 more diagnostics"));
    }

    #[test]
    fn test_summarize_for_prompt_empty() {
        let output = "No errors or warnings";
        let summary = DiagnosticsFilter::summarize_for_prompt(output, 5);
        assert_eq!(summary, "No diagnostics found.");
    }

    #[test]
    fn test_summarize_for_prompt_single() {
        let output = r#"error[E0012]: cannot find value `x` in this scope
   --> src/main.rs:10:5
    |
10  |     println!("{}", x);
    |                  ^ not found in this scope
"#;

        let summary = DiagnosticsFilter::summarize_for_prompt(output, 1);
        assert!(summary.contains("Found 1 diagnostic"));
        assert!(summary.contains("error"));
    }

    #[test]
    fn test_location_parsing() {
        let output = r#"error[E0012]: cannot find value `x` in this scope
   --> src/main.rs:10:5
    |
10  |     println!("{}", x);
    |                  ^ not found in this scope
"#;

        let entries = DiagnosticsFilter::parse_diagnostics(output);
        assert!(entries[0].location.is_some());
        let loc = entries[0].location.as_ref().unwrap();
        assert!(loc.contains("src/main.rs"));
        assert!(loc.contains("10"));
        assert!(loc.contains("5"));
    }

    #[test]
    fn test_code_extraction() {
        let output = r#"error[E0012]: cannot find value `x` in this scope
   --> src/main.rs:10:5
    |
10  |     println!("{}", x);
    |                  ^ not found in this scope
"#;

        let entries = DiagnosticsFilter::parse_diagnostics(output);
        assert_eq!(entries[0].code, Some("E0012".to_string()));
    }

    #[test]
    fn test_no_code_in_message() {
        let output = r#"error: cannot find value `x` in this scope
   --> src/main.rs:10:5
    |
10  |     println!("{}", x);
    |                  ^ not found in this scope
"#;

        let entries = DiagnosticsFilter::parse_diagnostics(output);
        assert_eq!(entries[0].code, None);
        assert_eq!(entries[0].level, "error");
    }

    #[test]
    fn test_message_extraction() {
        let output = r#"error[E0012]: cannot find value `x` in this scope
   --> src/main.rs:10:5
    |
10  |     println!("{}", x);
    |                  ^ not found in this scope
"#;

        let entries = DiagnosticsFilter::parse_diagnostics(output);
        assert_eq!(entries[0].message, "cannot find value `x` in this scope");
    }
}
