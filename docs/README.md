# Aaroneous: The Machine-Native Stem Cell Engine (Agent-Zero)

Aaroneous is a high-performance, binary-native synthetic intelligence engine designed to replace interpretive Python-based orchestration with compiled "Enzymatic" modules.

## Architectural Principles
- **Stem Cell Core:** Aaroneous is a minimal bootstrap binary (`a-run`) that materializes complex intelligence by composing binary chromosomes.
- **Enzymatic Execution:** Compute-intensive logic is moved from scripts to compiled WASM or Native DLL modules (Enzymes), communicating via zero-copy Shared Memory (The Synapse).
- **HOX Patterning:** Agent identity and capability expression are governed by positional roles defined in the `hox_map.json` registry.
- **Husk Model:** GGUF models are treated as static data assets (ore) to be mined and distilled into specialized binary execution paths.

## Components
- `bin/`: Machine-native executables (e.g., `a-run.exe`).
- `chromosomes/`: Compiled binary modules (Enzymes).
- `registry/`: HOX maps, Splicing recipes, Tensor indices, and SAB manifests/cache (`sab_*.json`, `sab_matrix.generated.json`).
- `include/`: AAS-ABI definitions (`aas_abi.h`).
- `src/`: Source code for the Host and Enzymes.

## SAB Matrix
- `registry/sab_matrix.json` is the baseline mapping.
- New `registry/sab_*.json` manifests are merged into the runtime matrix.
- `a-run` prefers `registry/sab_matrix.generated.json` and rebuilds it if manifests are newer.

## Operational Command
```powershell
# Run in console mode for debugging
./bin/a-run.exe --console

# Run as Windows Service
./bin/a-run.exe
```

## Federation Status
Aaroneous acts as the parent of the AAS Federation, capable of synthesizing hybridized offspring for specialized domains (Guild, Maelstrom, Merlin, etc.).
