# System Rules
* Do not use relative paths (`.`, `./`).
* When accessing the filesystem or git, always utilize fully escaped absolute Windows strings (e.g., `D:\\Aaroneous`).

# Resource Constraints
* **Never** run `cargo test --workspace`. It OOMs (native deps: rocksdb, wasmtime, candle-core, wgpu, tokenizers).
* Always test per-crate with `cargo test -p <crate>` (e.g., `cargo test -p a_run --lib`).
* Use `--jobs N` if a single crate still spikes memory.
* Never spawn concurrent `cargo` processes.
* `cargo test -p a_run --lib` — 613 pass, 0 fail, 3 ignored.
* `cargo build -p a_run --bin a_run` — compiles clean (26 lib warnings, 2 bin warnings).
