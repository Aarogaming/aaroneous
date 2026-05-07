# The Marionette Pipeline: Massive Open-Source Ingestion

The Marionette Pipeline is Aaroneous's strategy for achieving infinite capability expansion. Instead of building every feature from scratch or bloating the core Rust binary with thousands of dependencies, Aaroneous acts as an **Operating System for Intelligence**.

It "captures" open-source repositories, wraps them in a strict WebAssembly (WASM) sandbox, and loads them dynamically as Sovereign Agent Bundles (SABs).

## Architecture: The Universal SAB

Every open-source tool is normalized through a single interface: `aaroneous:sab/core`.

1. **`describe()`**: Returns a JSON schema of the tool's capabilities.
2. **`execute(command, payload)`**: The universal invocation method.

This completely abstracts the underlying implementation. The core Aaroneous system doesn't need to know if it's running a Dataframe engine (Polars), an HTML scraper (Thirtyfour), or a local shell (Nushell). 

## The "Capture & Wrap" Workflow

This workflow can be executed by human contributors or, eventually, autonomously by the `GenesisArchitect` and `Merlin` agents.

### Step 1: Discovery
Identify a high-value crate on crates.io or GitHub (e.g., `calamine` for parsing Excel files).

### Step 2: Instantiation
Clone the `templates/universal_sab` directory to create a new workspace (e.g., `plugins/excel_parser_sab`).

### Step 3: Glue Generation
Modify the `Cargo.toml` to include the target crate.
Modify `src/lib.rs` to route the `execute()` commands to the crate's API.
*Example:* `execute("read_sheet", "{\"file\": \"data.xlsx\"}")` -> `calamine::open_workbook(...)`.

### Step 4: Fabrication
Compile the workspace to WASM:
```bash
cargo build --target wasm32-wasip1 --release
```
Compress and package it alongside its `manifest.json` into a `.sovereign` artifact.

### Step 5: Digestion
The `ComponentRegistry` hot-loads the new `.sovereign` file.
The `OmniRelic` embeds the `manifest.json` description into the multidimensional semantic graph.

## Semantic Routing (Odin & Omni Relic)

Because we have a standardized JSON description of every plugin, we solve the routing problem mathematically rather than programmatically.

When the user asks Aaroneous to "Find the average revenue in this Excel file":
1. **Odin** embeds the intent.
2. **Omni Relic** performs a relativistic search, pulling the `.sab` modules mathematically closest to "parse Excel" and "calculate average".
3. **Odin** streams the payloads through the WASM boundary to the `excel_parser_sab` and `polars_sab` via NATS.
4. The core never crashes, the core never bloats.

## Security & Isolation

- **Zero-Blast-Radius**: A panic in an open-source crate only kills its localized WASM instance. The Aaroneous host catches the trap and respawns it.
- **Strict Capabilities**: WASM inherently denies file system and network access unless explicitly granted. A math plugin cannot exfiltrate data, even if the underlying open-source crate contains a malicious dependency.

*One for all, and all for one. Welcome to the Swarm.*