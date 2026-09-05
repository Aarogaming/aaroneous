// dev/tools/afc/src/repl.rs
use crate::gatekeeper::Gatekeeper;
use crate::git::GitEngine;
use crate::recipe::{DiagnosticsFilter, Step};
use crate::router::{
    ChatCompletionRequest, ChatMessage, ToolRegistry, TypedExtractor, TypedRouterClient,
};
use crate::state::{FlightState, StateMachine};
use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct ReplSummary {
    pub task_prompt: String,
    pub turns: usize,
    pub completed: bool,
    pub outcome_summary: String,
    pub tool_calls_executed: usize,
}

pub struct SovereignRepl {
    pub repo_root: PathBuf,
    pub client: TypedRouterClient,
    pub model_name: String,
}

impl SovereignRepl {
    pub fn new(
        repo_root: PathBuf,
        client: TypedRouterClient,
        model_name: impl Into<String>,
    ) -> Self {
        Self {
            repo_root,
            client,
            model_name: model_name.into(),
        }
    }

    /// Run the infinite/bounded sovereign REPL loop until completion or turn limit
    pub async fn run_autonomous_cycle(
        &self,
        task_prompt: &str,
        max_turns: usize,
        state_machine: &mut StateMachine,
    ) -> Result<ReplSummary> {
        let branch = GitEngine::current_branch(&self.repo_root)
            .await
            .unwrap_or_else(|_| "unknown".into());
        let initial_sha = GitEngine::head_commit_sha(&self.repo_root)
            .await
            .unwrap_or_else(|_| "unknown".into());
        let is_dirty = GitEngine::is_dirty(&self.repo_root).await.unwrap_or(false);

        info!("==========================================================");
        info!("          SOVEREIGN AUTONOMOUS REPL CYCLE START           ");
        info!("==========================================================");
        info!(
            "Target: {} | Branch: {} | SHA: {} | Dirty: {}",
            self.repo_root.display(),
            branch,
            initial_sha,
            is_dirty
        );

        let mut messages: Vec<ChatMessage> = vec![
            ChatMessage::system(
                "You are the Aaroneous Sovereign Autonomous Agent operating with local machine authority.\n\
                 Commands are executed exclusively via Git Bash (C:\\Program Files\\Git\\bin\\bash.exe).\n\
                 Tools available:\n\
                 - run_terminal(command): Execute shell instruction in repo root.\n\
                 - propose_patch(file_path, start_line, end_line, target_content, replacement_content, explanation): Apply code replacement.\n\
                 - report_defect(task_id, file_path, line_number, tier, defect_type, description): Append defect to queue.\n\
                 - complete_task(summary, status): Finish and report outcome.\n\
                 Rules:\n\
                 - Zero unwrap/panic or unsafe in Rust code.\n\
                 - Keep generation token-efficient.\n\
                 - Verify modifications using cargo check or test before signalling completion."
            ),
            ChatMessage::user(format!(
                "Repository Root: {}\nActive Branch: {}\nGit Status: {}\nTask: {}",
                self.repo_root.display(),
                branch,
                if is_dirty { "Modified (dirty)" } else { "Clean" },
                task_prompt
            )),
        ];

        let tools = ToolRegistry::sovereign_tools();
        let mut turns = 0;
        let mut completed = false;
        let mut outcome_summary = String::new();
        let mut tool_calls_executed = 0;

        while turns < max_turns && !completed {
            turns += 1;
            info!("[Sovereign REPL] >>> Turn {turns} of {max_turns} <<<");

            // 1. In-process Git state check & Axocoatl micro-state transition
            let dirty = GitEngine::is_dirty(&self.repo_root).await.unwrap_or(false);
            state_machine.transition_to(
                FlightState::IsolatedRemediation {
                    task_id: format!("repl_turn_{turns}"),
                    target_file: PathBuf::new(),
                    target_lines: (0, 0),
                    defect_description: format!("Turn {turns} execution"),
                    compiler_feedback: None,
                },
                format!("Turn {turns}: Autonomous execution (dirty: {dirty})"),
            )?;

            // 2. Query model via Rig TypedRouterClient
            let req = ChatCompletionRequest {
                model: self.model_name.clone(),
                messages: messages.clone(),
                temperature: Some(0.2),
                max_tokens: Some(2048),
                tools: Some(tools.clone()),
                response_format: None,
            };

            let response = match self.client.complete(&req).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("[Sovereign REPL] Model request error: {e}");
                    messages.push(ChatMessage::user(format!(
                        "Model communication error: {e}. Please retry."
                    )));
                    continue;
                }
            };

            // Save assistant response message in context
            if let Some(choice) = response.choices.first() {
                messages.push(choice.message.to_chat_message());
            }

            // 3. Catch tool call via TypedExtractor (supporting OpenAI tool_calls, XML, JSON fences)
            let tool_call_opt = TypedExtractor::extract_tool_call(&response)?;
            let tool_call = match tool_call_opt {
                Some(tc) => tc,
                None => {
                    let text = response
                        .choices
                        .first()
                        .and_then(|c| c.message.content.as_deref())
                        .unwrap_or("");
                    if text.to_lowercase().contains("complete")
                        || text.to_lowercase().contains("done")
                    {
                        completed = true;
                        outcome_summary = text.to_string();
                        info!("[Sovereign REPL] Completed based on direct model confirmation.");
                        break;
                    } else {
                        messages.push(ChatMessage::user(
                            "Please choose an action: run_terminal, propose_patch, or complete_task.",
                        ));
                        continue;
                    }
                }
            };

            tool_calls_executed += 1;
            info!(
                "[Sovereign REPL] Tool call: '{}' with arguments: {}",
                tool_call.name, tool_call.arguments
            );

            // 4. Execute tool via sovereign engines
            match tool_call.name.as_str() {
                "complete_task" => {
                    completed = true;
                    outcome_summary = tool_call
                        .arguments
                        .get("summary")
                        .and_then(|s| s.as_str())
                        .unwrap_or("Task completed successfully")
                        .to_string();

                    info!("[Sovereign REPL] Task Complete: {outcome_summary}");
                    state_machine.transition_to(
                        FlightState::Completed,
                        format!("REPL Turn {turns}: Completed: {outcome_summary}"),
                    )?;
                    break;
                }
                "run_terminal" => {
                    let cmd_str = tool_call
                        .arguments
                        .get("command")
                        .and_then(|c| c.as_str())
                        .unwrap_or("");

                    if cmd_str.trim().is_empty() {
                        messages.push(ChatMessage::tool(
                            "run_terminal",
                            "Error: Missing command parameter.",
                        ));
                        continue;
                    }

                    info!("[Sovereign REPL] Executing via Git Bash: {cmd_str}");
                    let step = Step::run_terminal(cmd_str, self.repo_root.clone());
                    let output = step.execute().await?;

                    let mut result_text = format!("Exit Code: {}\n", output.code);
                    if !output.raw_stdout.is_empty() {
                        result_text.push_str(&format!("STDOUT:\n{}\n", output.raw_stdout));
                    }
                    if !output.raw_stderr.is_empty() {
                        let filtered =
                            DiagnosticsFilter::summarize_for_prompt(&output.raw_stderr, 10);
                        result_text.push_str(&format!("STDERR:\n{}\n", filtered));
                    }

                    messages.push(ChatMessage::tool("run_terminal", result_text));
                }
                "propose_patch" => {
                    let file_path = tool_call
                        .arguments
                        .get("file_path")
                        .and_then(|f| f.as_str())
                        .unwrap_or("");
                    let target_content = tool_call
                        .arguments
                        .get("target_content")
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    let replacement = tool_call
                        .arguments
                        .get("replacement_content")
                        .and_then(|r| r.as_str())
                        .unwrap_or("");

                    let abs_path = self.repo_root.join(file_path);
                    let patch_feedback =
                        Self::apply_patch(&abs_path, target_content, replacement).await;

                    if patch_feedback.contains("applied cleanly") {
                        // Run validation gate
                        match Gatekeeper::check_workspace(&self.repo_root).await {
                            Ok(_) => messages.push(ChatMessage::tool(
                                "propose_patch",
                                format!("{patch_feedback}\nWorkspace compilation check: PASSED."),
                            )),
                            Err(e) => messages.push(ChatMessage::tool(
                                "propose_patch",
                                format!(
                                    "{patch_feedback}\nWorkspace compilation check: FAILED:\n{e}"
                                ),
                            )),
                        }
                    } else {
                        messages.push(ChatMessage::tool("propose_patch", patch_feedback));
                    }
                }
                "report_defect" => {
                    let desc = tool_call
                        .arguments
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    info!("[Sovereign REPL] Defect reported: {desc}");
                    messages.push(ChatMessage::tool(
                        "report_defect",
                        format!("Defect queued successfully: {desc}"),
                    ));
                }
                other => {
                    messages.push(ChatMessage::tool(
                        other,
                        format!("Unknown tool: '{other}'. Available: run_terminal, propose_patch, report_defect, complete_task."),
                    ));
                }
            }
        }

        if !completed {
            warn!(
                "[Sovereign REPL] Reached maximum turns ({max_turns}) without explicit completion."
            );
            outcome_summary = format!("Reached maximum turns ({max_turns})");
        }

        info!("==========================================================");
        info!("           SOVEREIGN AUTONOMOUS REPL CYCLE END            ");
        info!("==========================================================");

        Ok(ReplSummary {
            task_prompt: task_prompt.to_string(),
            turns,
            completed,
            outcome_summary,
            tool_calls_executed,
        })
    }

    async fn apply_patch(path: &Path, target: &str, replacement: &str) -> String {
        if !path.is_file() {
            return format!("Error: Target file '{}' does not exist.", path.display());
        }

        match tokio::fs::read_to_string(path).await {
            Ok(content) => {
                if !content.contains(target) {
                    return format!(
                        "Error: target_content not found in '{}'. Ensure exact verbatim match.",
                        path.display()
                    );
                }

                let new_content = content.replace(target, replacement);
                match tokio::fs::write(path, new_content).await {
                    Ok(_) => format!("Patch applied cleanly to '{}'.", path.display()),
                    Err(e) => format!("Failed to write to '{}': {e}", path.display()),
                }
            }
            Err(e) => format!("Failed to read file '{}': {e}", path.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_apply_patch_flow() {
        let temp_dir = std::env::temp_dir().join("afc_test_repl");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;
        let test_file = temp_dir.join("sample.txt");
        let _ = tokio::fs::write(&test_file, "line1\nold_content\nline3\n").await;

        let res = SovereignRepl::apply_patch(&test_file, "old_content", "new_content").await;
        assert!(res.contains("applied cleanly"));

        let updated = tokio::fs::read_to_string(&test_file)
            .await
            .unwrap_or_default();
        assert!(updated.contains("new_content"));
        assert!(!updated.contains("old_content"));

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
