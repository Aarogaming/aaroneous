# 13: Deconstruction & Reorganization Methodology

## The Core Philosophy: Non-Destructive Forensic Morphogenesis

Deconstructing a complex, multi-layered repository like Aaroneous cannot be done by chaotic ad-hoc folder moving. Random file moves break dependency graphs, corrupt build manifests, and create unorganized "junk drawers".

Instead, we employ the **4-Stage Morphogenesis Ledger & Staging Methodology**:

```
                       ┌─────────────────────────────────────────┐
                       │          1. TRIAGE & LEDGER             │
                       │ Categorize all files into 5 Destinations│
                       └────────────────────┬────────────────────┘
                                            │
                       ┌────────────────────▼────────────────────┐
                       │      2. ISOLATED STAGING & WRAP         │
                       │ Extract to dev/staging/, strip WASM/WIT,│
                       │ wire Machine-Native Linking Protocol    │
                       └────────────────────┬────────────────────┘
                                            │
                       ┌────────────────────▼────────────────────┐
                       │       3. GHOST & ARTIFACT PURGE         │
                       │ Delete empty agent shells & temp files  │
                       └────────────────────┬────────────────────┘
                                            │
                       ┌────────────────────▼────────────────────┐
                       │     4. RE-ASSIGNMENT & RE-PLACEMENT     │
                       │ Deploy clean crates to definitive paths │
                       │ & verify complete workspace compilation │
                       └─────────────────────────────────────────┘
```

---

## 🏛️ The 5 Sovereign Destination Buckets

Every file, module, and crate in Aaroneous is triaged into exactly one of the following 5 buckets:

### Bucket 1: `ORGANS` (First-Class Autonomous Programs)
Standalone, executable services running under the Process Supervisor:
- **`aaroneous_master`**: Core daemon, supervisor, shared memory synapse, and metabolic governor.
- **`marionette`**: User emulation, visual capture, process probing, and datalogging.
- **`chimera`**: Software adaptation, binary decompilation, AST mutation, and auto-patching.
- **`omni`**: 3D Galaxy semantic navigation and visual star-node search.
- **`specialists`**: Odin (Task DAGs), Merlin (Intelligence & Vector Index), Ariel (UI presentation).

### Bucket 2: `LIBRARIES` (Shared Machine Math & Common Primitives)
Pure Rust crates with zero circular dependencies:
- **`compute`**: Bayesian networks, Markov decision processes, game theory, linear algebra.
- **`paths` (`aaroneous_paths`)**: Centralized workspace path resolver.
- **`protocol`**: Machine-Native Linking Protocol binary headers, zero-copy structs, and opcodes.
- **`security`**: NLM intent classifier, token bucket rate limiter, and input sanitizers.

### Bucket 3: `UTILITIES & ADAPTERS` (Auto-Wrapped Tools)
External tools and libraries wrapped by the Stem Cell engine:
- Wrapped CLI tools (e.g. `rg_wrapper`, `ffmpeg_wrapper`, `git_wrapper`).
- Peripheral hooks (camera/audio stream adapters).

### Bucket 4: `GENETICS & DATA` (Static Models & State Registries)
Persistent data, neural weights, and genetic blueprints:
- GGUF neural weights (`genetics/q6k_only/`).
- LoRA adapters (`lora_adapter_vault/`).
- Trait databases (`hox.db/`, `hive.db/`).
- WGSL GPU compute shaders (`shaders/`).

### Bucket 5: `ARCHIVE & PURGE` (Dead Artifacts & Scaffolding)
To be deleted or archived in `dev/archive/`:
- Ghost folders (`agents/sleep_cycle/target/`, `agents/system_janitor/target/`, etc.).
- Deprecated WASM/WIT layers (`templates/universal_sab/`, `.wasm` files).
- One-off ad-hoc formula/PDF scraping scripts in `scripts/`.
- Conflicting legacy summary markdown files in `docs/`.

---

## 🛠️ Step-by-Step Execution Protocol

1. **Generate Triage Ledger**:
   - Run `dev/tools/maintenance/deconstruction_triage.py` to produce `dev/tools/maintenance/triage_ledger.json`.
2. **Purge Ghost Artifacts**:
   - Safely remove confirmed empty shells in `agents/` and leftover temporary build artifacts.
3. **Stage & Reconstruct Organs**:
   - Extract and reconstruct each core organ into `dev/staging/` with clean `Cargo.toml` manifests, stripping legacy WASM/WIT layers.
4. **Deploy & Unify Workspace**:
   - Move staged crates to definitive clean directories (`crates/aaroneous_master`, `crates/marionette`, `crates/chimera`, `crates/omni`, `crates/specialists/`).
   - Update the root `Cargo.toml` with the unified `[workspace] members` list.
   - Run `dev/tools/diagnostics/inspect_codebase.py` and `cargo check` to verify flawless compilation.
