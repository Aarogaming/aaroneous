$ErrorActionPreference = "Stop"
cargo run --package xtask -- gate
exit $LASTEXITCODE
