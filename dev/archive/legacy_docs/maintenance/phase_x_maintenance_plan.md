# Phase X: Repository Cleanup & Maintenance Plan

## 🎯 Objective
Establish systematic repository maintenance practices to prevent bloat, ensure system stability, and maintain a clean, maintainable codebase.

## 📊 Current Repository State

### Repository Bloat Analysis
- **Total Size**: ~188GB
- **Primary Bloat Sources**:
  - Object files in `target/debug/deps` (~1.76GB, ~24 copies)
  - Query cache files `query-cache.bin` (205MB bloat across May-June builds)
  - Dep-graph binaries (multiple versions)
  - Large model GGUFs (1GB+ each, production models)

### .gitignore Coverage ✅
- `target/` - Rust build artifacts (ignored)
- `*.gguf`, `*.bin`, `*.safetensors` - Model files (ignored)
- `bin/*.exe`, `bin/*.dll` - Compiled binaries (ignored)
- `*.db`, `*.sqlite` - Database files (ignored)
- `*.log`, `/cache/`, `/logs/` - Logs (ignored)
- `src/` - Legacy source (ignored, except `core/hypervisor/src/`)
- `data/models/`, `data/*.db` - Data files (ignored)

## 🧹 Maintenance Practices

### 1. Build Artifact Cleanup

#### Weekly Cleanup (Every Sunday)
```powershell
# Remove old .rlib, .pdb, and duplicate artifacts
Remove-Item -Path "target/debug/deps/*.rlib" -Force -ErrorAction SilentlyContinue
Remove-Item -Path "target/debug/deps/*.pdb" -Force -ErrorAction SilentlyContinue
# Remove old query-cache.bin versions
Get-ChildItem -Path "target/debug" -Filter "query-cache.bin" | Remove-Item -Force
```

#### Monthly Cleanup (First Monday of Month)
```powershell
# Remove old dep-graph binaries
Get-ChildItem -Path "target/debug" -Filter "dep-graph.*" | Remove-Item -Force
# Remove old object files
Get-ChildItem -Path "target/debug/deps" -Filter "*.o" | Remove-Item -Force
```

### 2. Model Management

#### Production Models (Externalize)
- **Criteria**: Models > 1GB in size
- **Action**: Move to external storage (Git LFS or separate repo)
- **Keep in Repo**: Development models < 500MB

#### Model Rotation
- **Weekly**: Review model usage, archive unused models
- **Monthly**: Prune models not used in 30+ days

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

## 📝 Notes

- **Maintenance Mode**: System is in maintenance mode until Phase X is complete
- **Development Resume**: Once system is stable, clean, and properly maintained
- **Documentation**: All new documentation goes to `/docs/` subfolders
- **Root Directory**: Only README.md, CHANGELOG.md, INDEX.md, AGENTS.md allowed

---

*Last Updated: Phase X Maintenance Mode | Status: 🟢 MAINTENANCE MODE*