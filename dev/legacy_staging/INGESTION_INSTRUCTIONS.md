# Legacy Ingestion Instructions

## Current Status

The legacy staging zone is ready at `dev/legacy_staging/`.

### Already Staged (Example)

- **batch_01_target/**: State tracker rebase example (complete workflow)
  - `legacy_state_tracker.py`: Synthetic legacy module demonstrating anti-patterns
  - `README.md`: Origin and rebase strategy documentation

## How to Clone Additional Legacy Repositories

### Step 1: Identify Target Repository

From your dashboard or AAS archives, identify the repository to ingest. Examples:
- `AaroneousAutomationSuite`
- `AutoWizard101`
- Any legacy Python/C++ modules from historical versions

### Step 2: Clone to Quarantine

```bash
cd dev/legacy_staging
git clone https://github.com/Aarogaming/<REPO_NAME>.git target_artifact
cd ../..
```

### Step 3: Identify Core Files

```bash
find dev/legacy_staging/target_artifact -maxdepth 3 -not -path '*/.*' | grep "\.py$"
# or for C++ files
find dev/legacy_staging/target_artifact -maxdepth 3 -not -path '*/.*' | grep "\.cpp$"
```

### Step 4: Stage Individual Modules

Don't ingest entire repositories. Stage only specific problematic modules:

```bash
cp dev/legacy_staging/target_artifact/src/module_name.py \
   dev/legacy_staging/batch_02_target/
```

### Step 5: Run Forensic Ingestion Sprint

Follow the RFC-0005 protocol documented in `AGENTS.md`:

1. **Stage**: Module is in `dev/legacy_staging/batch_NN_target/`
2. **Trace**: Use `dev/emulator_harness` to extract TraceEvents
3. **Document**: Create `docs/forensics/0003_<name>.md`
4. **Test**: Create `tests/negative_contracts/test_<name>_anti_patterns.rs`
5. **Synthesize**: Rebase to `crates/<new_crate>/src/lib.rs`
6. **Verify**: Run `bash scripts/agent_check.sh`

## Available Archives

Legacy builds are available in `dist/`:
- `Aaroneous-v0.1.0-windows-x86_64.zip` (v0.1.0 binary)
- `Aaroneous-v0.3.0-windows-x86_64.zip` (v0.3.0 binary)

These contain compiled binaries, not source code. To extract source:
1. Check GitHub releases for tagged source archives
2. Look for `.tar.gz` or `.zip` source tarballs in release notes
3. Contact the original repository maintainers for source access

## Next Actions

1. **Identify Source Repositories**: Determine which legacy repos need ingestion
2. **Clone to Staging**: Use the instructions above
3. **Analyze Core Modules**: Run `find` command to identify problematic files
4. **Initiate Ingestion Sprint**: Follow RFC-0005 protocol for each module

## References

- **Forensic Protocol**: `docs/rfc/RFC-0005-FORENSIC-INGESTION.md`
- **Agent Guidelines**: `AGENTS.md`
- **Verification Script**: `scripts/agent_check.sh`
- **Case Study Template**: `docs/forensics/0001_aas_omni_galaxy_view.md`

