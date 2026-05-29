# Detailed Vulnerability Report

## Critical Vulnerabilities (2)
1. **RUSTSEC-2026-0095**: Wasmtime with Winch compiler backend may allow a sandbox-escaping memory access
   - Severity: 9 (critical)
   - Solution: Upgrade to >=36.0.7, <37.0.0 OR >=42.0.2, <43.0.0 OR >=43.0.1

2. **RUSTSEC-2026-0096**: Miscompiled guest heap access enables sandbox escape on aarch64 Cranelift
   - Severity: 9 (critical)
   - Solution: Upgrade to >=36.0.7, <37.0.0 OR >=42.0.2, <43.0.0 OR >=43.0.1

## High Vulnerabilities (1)
1. **RUSTSEC-2026-0089**: Host panic when Winch compiler executes `table.fill`
   - Severity: 5.9 (medium)
   - Solution: Upgrade to >=36.0.7, <37.0.0 OR >=42.0.2, <43.0.0 OR >=43.0.1

## Medium Vulnerabilities (3)
1. **RUSTSEC-2026-0086**: Host data leakage with 64-bit tables and Winch
   - Severity: 2.3 (low)
   - Solution: Upgrade to >=36.0.7, <37.0.0 OR >=42.0.2, <43.0.0 OR >=43.0.1

2. **RUSTSEC-2026-0088**: Data leakage between pooling allocator instances
   - Severity: 2.3 (low)
   - Solution: Upgrade to >=36.0.7, <37.0.0 OR >=42.0.2, <43.0.0 OR >=43.0.1

3. **RUSTSEC-2026-0094**: Improperly masked return value from `table.grow` with Winch compiler backend
   - Severity: 6.1 (medium)
   - Solution: Upgrade to >=36.0.7, <37.0.0 OR >=42.0.2, <43.0.0 OR >=43.0.1

## Unmaintained Dependencies (3)
1. **bincode 1.3.3**: Unmaintained
   - ID: RUSTSEC-2025-0141
   - Dependency tree: bincode 1.3.3 → llama-gguf 0.14.0 → a_run 0.1.0

2. **fxhash 0.2.1**: No longer maintained
   - ID: RUSTSEC-2025-0057
   - Dependency tree: fxhash 0.2.1 → fxprof-processed-profile 0.6.0 → wasmtime 24.0.9 → a_run 0.1.0

3. **paste 1.0.15**: No longer maintained
   - ID: RUSTSEC-2024-0436
   - Dependency tree: paste 1.0.15 → wasmtime 24.0.9 → a_run 0.1.0

## Vulnerability Summary
- Total vulnerabilities: 377
- Critical: 2
- High: 1
- Medium: 3
- Unmaintained: 3