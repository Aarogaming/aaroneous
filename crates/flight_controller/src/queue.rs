// crates/flight_controller/src/queue.rs
use anyhow::{Context, Result};
use chrono::Local;
use regex::Regex;
use std::path::Path;
use tokio::fs;
use tracing::info;

pub struct QueueManager;

impl QueueManager {
    /// Extract titles of all pending `[ ]` tasks in the active audit queue.
    pub async fn find_pending_tasks(queue_path: &Path) -> Result<Vec<String>> {
        if !queue_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(queue_path)
            .await
            .context("Failed to read active audit queue file")?;

        let re = Regex::new(r"-\s*\[\s*\]\s*\*\*([^\*]+)\*\*")?;
        let mut tasks = Vec::new();

        for cap in re.captures_iter(&content) {
            if let Some(matched) = cap.get(1) {
                tasks.push(matched.as_str().trim().to_string());
            }
        }

        Ok(tasks)
    }

    /// Safely mark a specific task as completed `[x]` using literal string replacement.
    pub async fn mark_task_completed(queue_path: &Path, task_title: &str) -> Result<()> {
        if !queue_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(queue_path)
            .await
            .context("Failed to read active audit queue")?;

        let old_item = format!("- [ ] **{task_title}**");
        let new_item = format!("- [x] **{task_title}**");

        if content.contains(&old_item) {
            let updated = content.replace(&old_item, &new_item);
            fs::write(queue_path, updated)
                .await
                .context("Failed to write updated queue file")?;
            info!("Marked task completed in active queue: '{task_title}'");
        }

        Ok(())
    }

    /// Sweep all `[x]` tasks into REPAIR_LOG.md and CHANGELOG.md, then purge them from the queue.
    pub async fn sweep_completed_tasks(
        queue_path: &Path,
        repair_log_path: &Path,
        changelog_path: &Path,
    ) -> Result<usize> {
        if !queue_path.exists() {
            return Ok(0);
        }

        let content = fs::read_to_string(queue_path)
            .await
            .context("Failed to read active audit queue for sweep")?;

        let header_re = Regex::new(r"^-\s*\[x\]\s*\*\*([^\*]+)\*\*(.*)$")?;
        let task_start_re = Regex::new(r"^-\s*\[[ x]\]")?;

        let mut remaining_lines: Vec<String> = Vec::new();
        let mut repair_entries: Vec<String> = Vec::new();
        let mut changelog_entries: Vec<String> = Vec::new();

        let mut current_swept_title: Option<String> = None;
        let mut current_swept_details: Vec<String> = Vec::new();

        for line in content.lines() {
            if let Some(caps) = header_re.captures(line) {
                // Flush previous swept task if any
                if let Some(title) = current_swept_title.take() {
                    let details = current_swept_details.join("\n");
                    repair_entries.push(format!("- **{title}**\n  {details}"));
                    changelog_entries.push(format!(
                        "- **{title}**: Remediated and verified via autonomous audit cycle."
                    ));
                    current_swept_details.clear();
                }

                let title = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
                let remainder = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();
                if !remainder.is_empty() {
                    current_swept_details.push(remainder);
                }
                current_swept_title = Some(title);
            } else if current_swept_title.is_some() {
                // If this is a new top-level markdown task or header, end current swept task
                if task_start_re.is_match(line) || line.starts_with('#') {
                    if let Some(title) = current_swept_title.take() {
                        let details = current_swept_details.join("\n");
                        repair_entries.push(format!("- **{title}**\n  {details}"));
                        changelog_entries.push(format!(
                            "- **{title}**: Remediated and verified via autonomous audit cycle."
                        ));
                        current_swept_details.clear();
                    }
                    remaining_lines.push(line.to_string());
                } else {
                    // Indented details of the swept task
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        current_swept_details.push(trimmed.to_string());
                    }
                }
            } else {
                remaining_lines.push(line.to_string());
            }
        }

        // Flush final swept task if ended at EOF
        if let Some(title) = current_swept_title.take() {
            let details = current_swept_details.join("\n");
            repair_entries.push(format!("- **{title}**\n  {details}"));
            changelog_entries.push(format!(
                "- **{title}**: Remediated and verified via autonomous audit cycle."
            ));
        }

        let swept_count = repair_entries.len();
        if swept_count == 0 {
            return Ok(0);
        }

        let now = Local::now().format("%Y-%m-%d %H:%M").to_string();
        info!("Sweep: Archiving {swept_count} resolved task(s)");

        // 1. Append to REPAIR_LOG.md
        let repair_header = format!("\n### 🛠️ Batch Remediation Sweep [{now}]\n\n");
        let repair_block = format!("{repair_header}{}\n", repair_entries.join("\n\n"));
        if repair_log_path.exists() {
            let mut existing = fs::read_to_string(repair_log_path)
                .await
                .unwrap_or_default();
            existing.push_str(&repair_block);
            fs::write(repair_log_path, existing).await?;
        } else {
            fs::write(repair_log_path, repair_block).await?;
        }

        // 2. Sync to CHANGELOG.md under ## [Unreleased]
        if changelog_path.exists() {
            let changelog_content = fs::read_to_string(changelog_path).await.unwrap_or_default();
            if changelog_content.contains("## [Unreleased]") {
                let unreleased_block = format!(
                    "## [Unreleased]\n\n### 🛡️ Automated Audit Remediations [{now}]\n{}\n",
                    changelog_entries.join("\n")
                );
                let updated = changelog_content.replacen("## [Unreleased]", &unreleased_block, 1);
                fs::write(changelog_path, updated).await?;
            }
        }

        // 3. Write cleaned queue
        let cleaned_queue = remaining_lines.join("\n");
        fs::write(queue_path, cleaned_queue.trim()).await?;

        Ok(swept_count)
    }
}
