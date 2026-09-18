# Aaroneous Benchmarks

Performance benchmarks for Aaroneous core components. All latency and throughput claims in documentation must be validated here before being stated as facts.

## Running

```bash
# From workspace root
cd benches
cargo bench

# Run specific benchmark
cargo bench --bench ssm_inference
cargo bench --bench adaptation_latency
cargo bench --bench mcp_tool_dispatch
cargo bench --bench si_mount
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
| `si_mount` | `si_format` | `.si` cartridge memmap mount + magic validation | Pending |
| `ipc_throughput` | `ipc_bus` | SWMR ring buffer frames per second | Pending |

## Recording Results

When you run benchmarks on your machine, record the results here:

```
Machine: [CPU, RAM, OS]
Build: cargo build --release
Date: YYYY-MM-DD

Benchmark Name: [median] ns/op (± [stddev])
```
