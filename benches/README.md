# Aaroneous Benchmarks

Performance benchmarks for Aaroneous core components. All latency and throughput claims in documentation must be validated here before being stated as facts.

## Running

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench ssm_inference
cargo bench --bench adaptation_latency
cargo bench --bench mcp_tool_dispatch
```

## Setup

Add to workspace `Cargo.toml`:

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "ssm_inference"
harness = false

[[bench]]
name = "adaptation_latency"
harness = false

[[bench]]
name = "mcp_tool_dispatch"
harness = false
```

## Adding Benchmarks

1. Create a new file in `benches/` named `<component>_bench.rs`
2. Use criterion for statistical rigor
3. Document the machine, build profile, and inputs used
4. Update this README with the benchmark name and what it measures

## Benchmark Inventory

| Benchmark | Component | What it measures | Status |
|-----------|-----------|------------------|--------|
| `ssm_inference` | `compute` | Single-pass state-to-action inference latency | Pending |
| `adaptation_latency` | `autonomic_adaptation` | Error steering gradient step time | Pending |
| `mcp_tool_dispatch` | `capabilities` | UniversalTool JSON→latent→JSON round trip | Pending |
| `ipc_throughput` | `ipc_bus` | SWMR ring buffer frames per second | Pending |
| `memmap_load` | `si_format` | `.si` cartridge mount-to-ready time | Pending |

## Recording Results

When you run benchmarks on your machine, record the results here:

```
Machine: [CPU, RAM, OS]
Build: cargo build --release
Date: YYYY-MM-DD

Benchmark Name: [median] ns/op (± [stddev])
```
