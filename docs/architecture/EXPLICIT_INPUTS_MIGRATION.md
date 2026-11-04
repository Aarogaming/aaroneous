# Explicit Inputs Migration Guide: External-Control Entrypoints

> **Status**: Binding Architectural Guidance  
> **Applies to**: `crates/paths`, `crates/llm_gateway`, headless runners, external controllers, orchestration runners  
> **Related**: AGENTS.md Section 2 (Zero Ambient Authority), CRATIFY_SPEC Section 2 (Zero Ambient Authority)  
> **Lane / Packet**: Lane C81 / Packet P03  

---

## 1. Background & Rationale

Under Aaroneous architectural contracts (AGENTS.md Section 2 and CRATIFY_SPEC Section 2), library components and execution entrypoints must adhere to **Zero Ambient Authority**:
- All roots, working paths, model paths, endpoints, and credentials must arrive via explicit constructor injection.
- Components must never make ambient host assumptions, silently traverse parent directories looking for marker files (e.g. `Cargo.toml`), or query host user directories (`dirs::data_local_dir()`).
- External controllers and headless runners must never fall back to ambient default endpoints (`http://localhost:11434`), default model tags (`mistral:latest`), or hardcoded model file locations (`models/qwen2.5-1.5b.gguf`).

Implicit fallbacks introduce nondeterministic behavior across headless hosts, conceal configuration errors, and violate the sandboxing and determinism guarantees required for qualified operating-spine components.

Packet P03 / Lane C81 eliminates implicit path and model fallbacks from external-control entrypoints while preserving existing interactive developer workflows exclusively behind explicit bootstrap adapters.

---

## 2. Workspace Paths Migration (`crates/paths`)

### 2.1 The Issue with Legacy Discovery
Previously, `WorkspacePaths::discover()` or `WorkspacePaths::from_config(WorkspacePathsConfig::default())`:
1. Queried `std::env::current_exe()` and walked parent directories upward looking for `Cargo.toml`.
2. Fell back to `std::env::current_dir()`.
3. Queried ambient OS data directories via `dirs::data_local_dir()`.

In headless and containerized environments, this could bind to unintended host folders or fail nondeterministically.

### 2.2 Strict Constructor: `WorkspacePaths::strict` & `from_strict_config`
For external controllers and headless executions, callers must provide explicit paths:

```rust
use paths::{StrictWorkspacePathsConfig, WorkspacePaths, StrictPathsError};
use std::path::PathBuf;

// Construct explicit, validated paths
let paths = WorkspacePaths::strict(
    PathBuf::from("/var/lib/aaroneous/workspace"),
    PathBuf::from("/tmp/aaroneous"),
    PathBuf::from("/var/lib/aaroneous/data"),
)?;
```

Alternatively, validate a `WorkspacePathsConfig` into a `StrictWorkspacePathsConfig`:

```rust
use paths::{WorkspacePathsConfig, WorkspacePaths, StrictPathsError};

let config = WorkspacePathsConfig {
    root: Some(root_path),
    temp_dir: Some(temp_path),
    data_root: Some(data_path),
    ..Default::default()
};

let strict_config = config.validate_strict()?;
let paths = WorkspacePaths::from_strict_config(&strict_config);
```

### 2.3 Strict Error Reference (`StrictPathsError`)
| Error Variant | Cause |
|---|---|
| `StrictPathsError::MissingRoot` | `root` path was omitted or empty |
| `StrictPathsError::RootNotFound(p)` | Configured `root` directory does not exist on disk |
| `StrictPathsError::RootNotDirectory(p)` | Configured `root` exists but is a regular file |
| `StrictPathsError::MissingTempDir` | `temp_dir` was omitted or empty |
| `StrictPathsError::MissingDataRoot` | `data_root` was omitted or empty |
| `StrictPathsError::DataRootNotFound(p)` | Configured `data_root` does not exist on disk |
| `StrictPathsError::DataRootNotDirectory(p)` | Configured `data_root` exists but is a regular file |

### 2.4 Retained Developer Adapter: `WorkspacePaths::bootstrap_adapter`
For interactive developer sessions, manual debugging, and test environments where ambient discovery is explicitly intended, use:

```rust
// Explicitly opted into legacy ambient discovery:
let paths = WorkspacePaths::bootstrap_adapter()?;
```

---

## 3. LLM Gateway Migration (`crates/llm_gateway`)

### 3.1 The Issue with Legacy Construction
Previously, `LLMClient::new(LLMConfig)` silently injected default values when fields were missing:
- `BackendType::Local`: defaulted endpoint to `http://localhost:11434`, model to `mistral:latest`.
- `BackendType::GGUF`: defaulted model path to `models/qwen2.5-1.5b.gguf`.
- `BackendType::OpenAI`: defaulted base URL to `https://api.openai.com/v1`.
- `BackendType::Gemini`: defaulted base URL to Google API endpoints.

If a caller forgot to configure a model or endpoint, the client would silently attempt connections to default ports or paths, failing with obscure network errors or loading arbitrary local models.

### 3.2 Strict Constructor: `LLMClient::new_strict`
All external controllers, supervisors, and headless services must use `LLMClient::new_strict`:

```rust
use llm_gateway::{LLMClient, LLMConfig, BackendType, StrictLLMError};

let config = LLMConfig {
    backend: BackendType::Local,
    endpoint: Some("http://127.0.0.1:8000/v1".to_string()),
    model: Some("Qwen/Qwen2.5-Coder-14B-Instruct-AWQ".to_string()),
    ..Default::default()
};

// Returns Err(StrictLLMError) if any required field is missing or invalid:
let client = LLMClient::new_strict(config)?;
```

### 3.3 Strict Error Reference (`StrictLLMError`)
| Error Variant | Cause |
|---|---|
| `StrictLLMError::MissingModel` | `model` was omitted or empty for backends requiring a model |
| `StrictLLMError::MissingModelPath` | `model_path` was omitted for `BackendType::GGUF` |
| `StrictLLMError::ModelPathNotFound(p)` | Specified GGUF model path does not exist on disk |
| `StrictLLMError::ModelPathIsDirectory(p)` | Specified GGUF model path is a directory, not a model file |
| `StrictLLMError::MissingEndpoint` | `endpoint` was omitted or empty for `BackendType::Local` |
| `StrictLLMError::InvalidEndpoint(e)` | `endpoint` failed URL format validation |
| `StrictLLMError::MissingApiKey` | API key was omitted or empty for cloud providers (`OpenAI`, `Anthropic`, `Gemini`) |
| `StrictLLMError::MissingBaseUrl` | Base URL was explicitly empty or invalid |
| `StrictLLMError::ProviderInitialization(e)` | Underlying provider constructor failed |

### 3.4 Retained Developer Adapter: `LLMClient::bootstrap_adapter`
For legacy CLI entrypoints or local manual experimentation:

```rust
// Explicitly opted into legacy ambient default endpoints/models:
let client = LLMClient::bootstrap_adapter(config)?;
```

### 3.5 Model Environment Detection Migration
In `llm_gateway::model_environment`:
- External controllers must use `ModelEnvironmentDetector::scan_strict(&StrictModelEnvironmentConfig)`. Callers must supply explicit hub paths and root search directories.
- Legacy discovery (scanning ambient home directory models, Ollama default ports, LM Studio endpoints) is available solely via `ModelEnvironmentDetector::bootstrap_scan()`.

---

## 4. Caller Migration Summary Table

| Domain | Legacy / Ambient API | Strict External-Control API | Opt-in Compatibility Adapter |
|---|---|---|---|
| Paths | `WorkspacePaths::discover()` | `WorkspacePaths::strict(...)` | `WorkspacePaths::bootstrap_adapter()` |
| Paths Config | `WorkspacePaths::from_config(cfg)` | `cfg.validate_strict()`, `WorkspacePaths::from_strict_config(...)` | `WorkspacePaths::from_config(cfg)` (legacy) |
| LLM Client | `LLMClient::new(cfg)` | `LLMClient::new_strict(cfg)` | `LLMClient::bootstrap_adapter(cfg)` |
| LLM Config | Implicit defaults applied | `cfg.validate_strict()` | `cfg` with unvalidated fields |
| Model Environment | `detector.scan()` | `detector.scan_strict(&strict_cfg)` | `detector.bootstrap_scan()` |

---

## 5. Verification & Testing

Both `crates/paths` and `crates/llm_gateway` provide exhaustive unit tests verifying:
1. Missing required fields produce exact `StrictPathsError` / `StrictLLMError` variants.
2. Non-existent directories and paths fail with typed errors without searching parent directories.
3. Explicit valid configurations instantiate correctly without ambient defaults.
4. Legacy compatibility adapters remain functional for explicit bootstrap use cases.
