# Maintenance Guide

## 🎯 Purpose
This guide outlines the maintenance practices and procedures for the Aaroneous repository to ensure system stability, prevent bloat, and maintain a clean, maintainable codebase.

## 📊 Repository Health Status

### Current Status: 🟢 MAINTENANCE MODE
- **Phase**: Phase X Repository Cleanup & Maintenance
- **Target**: System Stability & Clean Repository
- **Repository Size**: ~176GB (needs cleanup)
- **Build Artifacts**: Cleaned (target/debug/deps)
- **Documentation**: 50+ files (80% complete)

### Repository Bloat Hotspots
1. **Object files** in `target/debug/deps` (~1.76GB)
2. **Query cache** files `query-cache.bin` (205MB)
3. **Dep-graph binaries** (multiple versions)
4. **Production models** (1GB+ GGUFs)

## 🧹 Maintenance Practices

### 1. Build Artifact Cleanup

#### What Gets Cleaned
- **Object files** (`*.o`) - Never commit these
- **Library files** (`*.rlib`) - Build artifacts
- **Debug symbols** (`*.pdb`) - Build artifacts
- **Query cache** (`query-cache.bin`) - Cache bloat
- **Dep-graph** (`dep-graph.*`) - Build artifacts
- **Target directory** (`target/`) - All Rust build artifacts

#### Cleanup Frequency
- **Weekly**: Remove old .rlib, .pdb, duplicate artifacts
- **Monthly**: Remove old dep-graph, object files
- **On-Demand**: After major build sessions

#### Cleanup Commands
```powershell
# Weekly cleanup
Remove-Item -Path "target/debug/deps/*.rlib" -Force -ErrorAction SilentlyContinue
Remove-Item -Path "target/debug/deps/*.pdb" -Force -ErrorAction SilentlyContinue
Get-ChildItem -Path "target/debug" -Filter "query-cache.bin" | Remove-Item -Force

# Monthly cleanup
Get-ChildItem -Path "target/debug" -Filter "dep-graph.*" | Remove-Item -Force
Get-ChildItem -Path "target/debug/deps" -Filter "*.o" | Remove-Item -Force
```

### 2. Model Management

#### Model Location
- **Primary Source**: `genetics\gguf_sources\`
- **Purpose**: All models for dissection, hybridization, and study

#### Model Categories
- **Source Models**: Models in `genetics\gguf_sources\` (keep in repo)
- **Processed Models**: Models in `genetics\q6k_only\` (keep in repo)
- **Experimental**: Archive to `docs/history/`

#### Model Rotation
- **Weekly**: Review model usage
- **Monthly**: Prune models not used in 30+ days
- **Quarterly**: Full model audit

### 3. Documentation Maintenance

#### Documentation Audit (Monthly)
- **Frequency**: First Friday of each month
- **Tasks**:
  - Audit all documentation claims against actual system state
  - Update status indicators (Phase X, Phase IV, etc.)
  - Archive superseded documents to `docs/history/`
  - Remove outdated references (week/day references, superseded phases)

#### Documentation Grounding
- **Pre-Commit Checklist**:
  - [ ] Claims are verifiable via system testing
  - [ ] Metrics are current (not aspirational)
  - [ ] File paths are accurate (reflect new `/docs/` locations)
  - [ ] No week/day references (achievement-based structure)

#### Documentation Rules
- **New Documentation**: Always write to `/docs/` subfolders
- **Root Directory**: Only README.md, CHANGELOG.md, INDEX.md, AGENTS.md
- **Superseded Docs**: Archive to `docs/history/`
- **Documentation Index**: Update INDEX.md after adding new files

### 4. Git Hygiene

#### Pre-Commit Checks
```powershell
# Check for build artifacts
Get-ChildItem -Path "target" -Recurse -File | Measure-Object | Select-Object Count
Get-ChildItem -Path "target" -Recurse -Filter "*.o" | Measure-Object | Select-Object Count
Get-ChildItem -Path "target" -Recurse -Filter "query-cache.bin" | Measure-Object | Select-Object Count
```

#### Commit Guidelines
- **Never commit**: Build artifacts, models, logs, cache
- **Always commit**: Source code, documentation, configuration
- **Use Git LFS**: For large files > 100MB

#### .gitignore Coverage
- ✅ `target/` - Rust build artifacts
- ✅ `*.gguf`, `*.bin`, `*.safetensors` - Model files
- ✅ `bin/*.exe`, `bin/*.dll` - Compiled binaries
- ✅ `*.db`, `*.sqlite` - Database files
- ✅ `*.log`, `/cache/`, `/logs/` - Logs
- ✅ `src/` - Legacy source
- ✅ `data/models/`, `data/*.db` - Data files

### 5. Repository Size Monitoring

#### Size Thresholds
- **Warning**: > 100GB (review cleanup schedule)
- **Critical**: > 150GB (immediate cleanup required)
- **Target**: < 50GB (ideal repository size)

#### Monitoring Script
```powershell
# Check repository size
$repoSize = (Get-Item -Path ".\." -Force).Length / 1GB
Write-Host "Repository size: $($repoSize.ToString('F2')) GB"

# Check for bloat
$targetSize = (Get-ChildItem -Path "target" -Recurse -File).Length / 1GB
Write-Host "Target directory size: $($targetSize.ToString('F2')) GB"
```

## 📅 Maintenance Schedule

### Weekly (Every Sunday)
- [ ] Build artifact cleanup (target/debug/deps)
- [ ] Model usage review
- [ ] Documentation status update
- [ ] Repository size check

### Monthly (First Monday)
- [ ] Deep cleanup (dep-graph, object files)
- [ ] Model rotation and archival
- [ ] Documentation audit
- [ ] .gitignore effectiveness review

### Quarterly (First Friday)
- [ ] Full repository audit
- [ ] Documentation completeness review
- [ ] Maintenance procedure effectiveness review
- [ ] Stakeholder sign-off on repository health

## 🔄 Maintenance Workflow

### 1. Pre-Development
- [ ] Check repository size
- [ ] Review .gitignore exclusions
- [ ] Verify no build artifacts in working directory

### 2. During Development
- [ ] Commit only source code and documentation
- [ ] Use Git LFS for large files
- [ ] Never commit build artifacts

### 3. Post-Development
- [ ] Run cleanup script
- [ ] Verify repository size
- [ ] Update documentation if needed

### 4. Pre-Commit
- [ ] Run pre-commit checks
- [ ] Verify no build artifacts
- [ ] Update documentation status

### 5. Pre-Deployment
- [ ] Full repository audit
- [ ] Documentation completeness check
- [ ] Maintenance procedure verification

## 🛠️ Maintenance Tools

### PowerShell Scripts
- `scripts/cleanup.ps1` - Build artifact cleanup
- `scripts/audit.ps1` - Repository audit
- `scripts/size-check.ps1` - Repository size monitoring

### Git Hooks
- `pre-commit` - Check for build artifacts
- `pre-push` - Repository size check

### Monitoring
- Repository size dashboard
- Build artifact tracking
- Model usage metrics

## 📈 Success Metrics

### Repository Health
- **Target Size**: < 50GB
- **Build Artifacts**: < 5% of total size
- **Documentation Coverage**: > 90%

### Maintenance Effectiveness
- **Cleanup Frequency**: Weekly (100% compliance)
- **Documentation Accuracy**: > 95%
- **Git Hygiene**: 100% (no build artifacts in commits)

### System Stability
- **Build Time**: < 5 minutes (after cleanup)
- **Repository Cloning**: < 10 minutes
- **CI/CD Pipeline**: < 15 minutes

## 🚀 Next Steps

### Immediate (This Week)
1. [ ] Create cleanup scripts
2. [ ] Set up Git hooks
3. [ ] Document maintenance procedures
4. [ ] Train team on maintenance practices

### Short-term (This Month)
1. [ ] Implement weekly cleanup schedule
2. [ ] Externalize production models
3. [ ] Complete Phase X documentation
4. [ ] Verify maintenance procedures

### Long-term (This Quarter)
1. [ ] Achieve target repository size
2. [ ] Complete Phase IV (production readiness)
3. [ ] Resume Phase 10-15 development
4. [ ] Establish maintenance cadence

## 📝 Important Notes

### Maintenance Mode
- **Current Status**: System is in maintenance mode until Phase X is complete
- **Development Resume**: Once system is stable, clean, and properly maintained
- **Documentation**: All new documentation goes to `/docs/` subfolders
- **Root Directory**: Only README.md, CHANGELOG.md, INDEX.md, AGENTS.md allowed

### Repository Bloat Prevention
- **Use .gitignore**: Never commit build artifacts
- **Use Git LFS**: For large files > 100MB
- **Regular Cleanup**: Weekly build artifact cleanup
- **Model Management**: Externalize production models

### Documentation Best Practices
- **Achievement-Based**: No week/day references
- **Grounded**: Claims must be verifiable
- **Accurate**: Metrics must be current
- **Organized**: All docs in `/docs/` subfolders

---

*Last Updated: Phase X Maintenance Mode | Status: 🟢 MAINTENANCE MODE*