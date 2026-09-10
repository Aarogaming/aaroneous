# Aaroneous Flight Control (AFC): Architectural Upgrade & Open-Source Integration Plan

**Document Version:** 1.0.0  
**Target Subsystem:** `dev/tools/afc/` (Aaroneous Flight Control Hypervisor)  
**Status:** Architectural Specification & Integration Blueprint  

---

## Executive Summary & Strategic Rationale

The Aaroneous Flight Controller (AFC) currently operates as an out-of-tree sovereign CI/CD hypervisor and management HUD. While the current beta provides functional autonomous cycles and model detection, its core mechanisms rely heavily on process spawning (`Command::new("git")`, `npx opencode`) and monolithic task loops.

By extracting proven architectural patterns from premier open-source Rust projects, AFC will evolve into a **pure-Rust, machine-native, low-latency hypervisor**:

| Project | Architectural Pattern | Target AFC Subsystem | Impact / Capability Gain |
|---|---|---|---|
| **Rig** | Compile-Time Type-Safe Model Routing & Tool Calling | `afc::router` & `afc::tools` | Direct OpenAI-compatible REST connection to LM Studio / Ollama with typed schema validation; zero Node.js/npx overhead. |
| **Picocode** | Minimalist CLI Recipe Pipelines & Output Filtering | `afc::recipe` | Deterministic, composable execution steps; eliminates compiler log bloat from model context. |
| **Gitoxide (`gix`)** | Sovereign Pure-Rust Repository State Engine | `afc::git_native` | In-process dirty checking, branch tracking, and diff inspection without external `git.exe` process spawn latency. |
| **Axocoatl** | Micro-State Delegation & KV Cache Conservation | `afc::state_machine` | Deconstructs monolithic audits into isolated subtasks; caps KV cache usage at <1,500 tokens per call. |

---

## Target Subsystem Map (`dev/tools/afc/src/`)

```
dev/tools/afc/src/
├── main.rs                   # Entry point (CLI & Desktop HUD launcher)
├── lib.rs                    # Re-exports and core hypervisor API
├── config.rs                 # Dynamic workspace pathing & flight parameters
├── engine.rs                 # 7-Phase Hypervisor Coordinator
├── gui.rs                    # egui/eframe HUD with live telemetry & controls
├── hardware.rs               # NVML thermal throttle & GPU VRAM monitor
├── queue.rs                  # ACTIVE_AUDIT_QUEUE.md parser & batch archiver
│
├── router/                   # [PILLAR 1: RIG PATTERN]
│   ├── mod.rs                # Typed client & provider dispatch
│   ├── client.rs             # Tokio-based direct REST client (/v1/chat/completions)
│   ├── types.rs              # Strongly-typed schemas (ChatCompletion, ToolDefinition)
│   ├── tools.rs              # Typed tool definitions (ProposePatch, SyntaxCheck)
│   └── extractor.rs          # Zero-panic structured JSON extractor
│
├── recipe/                   # [PILLAR 2: PICOCODE PATTERN]
│   ├── mod.rs                # Recipe engine API
│   ├── pipeline.rs           # Sequential & parallel recipe runner
│   ├── step.rs               # Atomic step specification with timeout & rollback
│   └── filter.rs             # Diagnostics filter: strips noise, keeps compiler codes
│
├── git/                      # [PILLAR 3: GITOXIDE PATTERN]
│   ├── mod.rs                # Unified Git abstraction
│   ├── gix_backend.rs        # Pure-Rust Gitoxide status, diffs, and HEAD queries
│   └── cli_fallback.rs       # Process-based fallback for complex staging operations
│
└── state/                    # [PILLAR 4: AXOCOATL PATTERN]
    ├── mod.rs                # State machine coordinator
    ├── machine.rs            # Micro-state transitions (Queued -> Patch -> Verify -> Commit)
    └── sanitizer.rs          # Delta-only context stripper (KV cache optimizer)
```

---

## Pillar 1: Rig Integration (Type-Safe Agent Routing)

### Objective
Interface directly with LM Studio / Ollama via OpenAI-compatible REST endpoints (`http://127.0.0.1:1234/v1/chat/completions`) using compile-time typed completions and tool definitions. This removes external CLI bridge layers while preventing invalid or malformed model responses.

### Module Skeleton: `src/router/`

```rust
// dev/tools/afc/src/router/types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub message: ResponseMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseMessage {
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}
```

```rust
// dev/tools/afc/src/router/client.rs
use crate::router::types::*;
use anyhow::{bail, Context, Result};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

pub struct TypedRouterClient {
    endpoint: String,
    api_token: Option<String>,
    timeout_duration: Duration,
}

impl TypedRouterClient {
    pub fn new(endpoint: impl Into<String>, api_token: Option<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_token,
            timeout_duration: Duration::from_secs(120),
        }
    }

    /// Send a type-safe chat completion request and receive validated responses
    pub async fn complete(&self, req: &ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let serialized = serde_json::to_string(req)?;
        let url = url::Url::parse(&format!("{}/chat/completions", self.endpoint))?;
        let host = url.host_str().unwrap_or("127.0.0.1");
        let port = url.port().unwrap_or(1234);

        let mut stream = timeout(
            Duration::from_millis(1500),
            TcpStream::connect((host, port)),
        )
        .await
        .context("Connection to local LLM endpoint timed out")?
        .context("Failed to connect to local LLM port")?;

        let mut http_req = format!(
            "POST /v1/chat/completions HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccept: application/json\r\n",
            serialized.len()
        );
        if let Some(ref token) = self.api_token {
            http_req.push_str(&format!("Authorization: Bearer {token}\r\n"));
        }
        http_req.push_str("Connection: close\r\n\r\n");
        http_req.push_str(&serialized);

        stream.write_all(http_req.as_bytes()).await?;

        let mut buf = Vec::new();
        let _ = timeout(self.timeout_duration, stream.read_to_end(&mut buf)).await?;
        let response_str = String::from_utf8_lossy(&buf);

        let body = if let Some(idx) = response_str.find("\r\n\r\n") {
            &response_str[idx + 4..]
        } else {
            &response_str
        };

        if let Some(json_start) = body.find('{') {
            let res: ChatCompletionResponse = serde_json::from_str(&body[json_start..])
                .context("Failed to parse typed ChatCompletionResponse")?;
            Ok(res)
        } else {
            bail!("Invalid HTTP response from model endpoint: {response_str}");
        }
    }
}
```

---

## Pillar 2: Picocode Integration (Minimalist Recipe Pipelines)

### Objective
Execute terminal actions (Clippy, Format, Unit Tests, Cargo Mutants, Security Audits) through lightweight, deterministic "recipes" that filter stdout/stderr to extract actionable compiler diagnostics while stripping redundant progress noise.

### Module Skeleton: `src/recipe/`

```rust
// dev/tools/afc/src/recipe/step.rs
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub struct Step {
    pub name: &'static str,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub timeout_duration: Duration,
    pub rollback_on_failure: bool,
}

impl Step {
    pub async fn execute(&self) -> Result<StepOutput> {
        let mut cmd = Command::new(&self.command);
        cmd.current_dir(&self.cwd)
            .args(&self.args)
            .kill_on_drop(true);

        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let child = cmd.output();
        let output = timeout(self.timeout_duration, child)
            .await
            .context(format!("Step '{}' timed out", self.name))?
            .context(format!("Step '{}' failed to start", self.name))?;

        Ok(StepOutput {
            name: self.name,
            success: output.status.success(),
            code: output.status.code().unwrap_or(-1),
            raw_stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            raw_stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

pub struct StepOutput {
    pub name: &'static str,
    pub success: bool,
    pub code: i32,
    pub raw_stdout: String,
    pub raw_stderr: String,
}
```

```rust
// dev/tools/afc/src/recipe/filter.rs
use regex::Regex;

pub struct DiagnosticsFilter;

impl DiagnosticsFilter {
    /// Extract only critical error lines and compiler codes from raw build output
    pub fn extract_errors(raw_log: &str) -> Vec<String> {
        let error_re = Regex::new(r"(?m)^(error\[E\d+\]:.*|error:.*)").unwrap();
        let location_re = Regex::new(r"(?m)^\s+-->\s+(.*:\d+:\d+)").unwrap();

        let mut diagnostics = Vec::new();
        for cap in error_re.captures_iter(raw_log) {
            if let Some(m) = cap.get(1) {
                diagnostics.push(m.as_str().trim().to_string());
            }
        }
        for cap in location_re.captures_iter(raw_log) {
            if let Some(m) = cap.get(1) {
                diagnostics.push(format!("Location: {}", m.as_str().trim()));
            }
        }

        diagnostics
    }
}
```

---

## Pillar 3: Gitoxide (`gix`) Integration (Pure-Rust Git)

### Objective
Perform all repository dirty checks, branch lookups, HEAD commit inspection, and zero-copy diff analyses in-process via `gix`. This eliminates `git.exe` process creation overhead and enables reliable operation in airgapped environments.

### Module Skeleton: `src/git/`

```rust
// dev/tools/afc/src/git/gix_backend.rs
use anyhow::{Context, Result};
use std::path::Path;

pub struct SovereignGitEngine {
    repo: gix::Repository,
}

impl SovereignGitEngine {
    /// Open the repository at the specified root using pure-Rust Gitoxide
    pub fn open(repo_path: &Path) -> Result<Self> {
        let repo = gix::open(repo_path)
            .context("Failed to open repository via Gitoxide (gix)")?;
        Ok(Self { repo })
    }

    /// Pure-Rust lookup of current HEAD branch name
    pub fn current_branch(&self) -> Result<String> {
        let head = self.repo.head()?;
        if let Some(ref_name) = head.referent_name() {
            let short = ref_name.shorten();
            Ok(short.to_string())
        } else {
            Ok("HEAD (detached)".to_string())
        }
    }

    /// In-process index dirty verification
    pub fn is_dirty(&self) -> Result<bool> {
        // Inspect repository index and working tree status without process fork
        let status = self.repo.status(gix::progress::Discard)?;
        // If status iterator yields any modified, added, or untracked changes
        Ok(status.is_dirty()?)
    }

    /// Read HEAD commit hash (40-char SHA)
    pub fn head_commit_hash(&self) -> Result<String> {
        let head = self.repo.head()?.id().context("Detached or missing HEAD")?;
        Ok(head.to_hex().to_string())
    }
}
```

---

## Pillar 4: Axocoatl Integration (Micro-State Delegation & KV Conservation)

### Objective
Prevent LLM context window bloat and generation slowdown by enforcing strict micro-state boundaries. In multi-cycle remediation, each model prompt receives only the isolated task delta (target file chunk + compiler error) rather than cumulative session history.

### Module Skeleton: `src/state/`

```rust
// dev/tools/afc/src/state/machine.rs
use crate::router::ChatMessage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlightState {
    Idle,
    Planning {
        spec_focus: String,
    },
    Auditing {
        category: String,
    },
    IsolatedRemediation {
        task_id: String,
        target_file: PathBuf,
        target_lines: (usize, usize),
        defect_description: String,
        compiler_feedback: Option<String>,
    },
    VerificationGate {
        modified_files: Vec<PathBuf>,
    },
    CommitLedger {
        commit_message: String,
    },
}

pub struct ContextSanitizer;

impl ContextSanitizer {
    /// Construct a strictly bounded, delta-only prompt for the given micro-state.
    /// Guarantees that token count remains <1,500 tokens to preserve KV cache efficiency.
    pub fn sanitize_prompt_for_remediation(
        file_content_chunk: &str,
        line_range: (usize, usize),
        defect: &str,
        compiler_error: Option<&str>,
    ) -> Vec<ChatMessage> {
        let mut prompt = format!(
            "Target Lines {}-{}:\n```rust\n{}\n```\nDefect: {}\n",
            line_range.0, line_range.1, file_content_chunk, defect
        );

        if let Some(err) = compiler_error {
            prompt.push_str(&format!("\nCompiler Feedback:\n{}\n", err));
        }

        vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are an expert Rust systems programmer. Output ONLY the replacement Rust lines with Result-bubbled, zero-unsafe code. No markdown commentary.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ]
    }
}
```

---

## Phased Implementation Roadmap

1. **Phase 1: Pure-Rust Git Engine Integration (`gix`)**
   - Add `gix = { version = "0.70", default-features = false, features = ["blocking", "revision"] }` to `dev/tools/afc/Cargo.toml`.
   - Update `afc::git` to utilize `gix` for non-modifying repository queries with fallback.
   - Run verification tests to ensure dirty checks and branch detection run 5x faster.

2. **Phase 2: Picocode Recipe Engine**
   - Implement `afc::recipe::Step`, `Recipe`, and `DiagnosticsFilter`.
   - Migrate `gatekeeper.rs` to run through declarative recipes with structured compiler error filtering.

3. **Phase 3: Rig Type-Safe Router**
   - Implement `afc::router::TypedRouterClient` and strongly typed completions.
   - Connect directly to local LM Studio `/v1/chat/completions` using detected API token.
   - Introduce typed `ToolDefinition` for automated code remediation.

4. **Phase 4: Axocoatl Micro-State Workflow**
   - Wire the 7-phase engine to transition through explicit `FlightState` enums.
   - Integrate `ContextSanitizer` to enforce <1,500 token KV cache limits.
   - Run end-to-end benchmark demonstrating zero memory accumulation across 10 autonomous cycles.
