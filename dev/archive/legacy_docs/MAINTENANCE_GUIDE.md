# Documentation Maintenance Guide

## 📚 How to Manage Documentation Going Forward

---

## 🎯 Principles

### Additive Approach ✅
- **Never delete progress documentation** - All work is valuable
- **Add new files to appropriate directories** - Maintain organization
- **Update indexes when adding new files** - Keep navigation current

### Category-Based Organization ✅
- Place each file in the most appropriate `docs/` subdirectory
- Use descriptive, consistent naming conventions
- Include phase numbers for chronological organization

---

## 📁 Directory Structure

```
docs/
├── architecture/      - System architecture documentation
├── deployment/       - Deployment procedures and guides
├── operations/       - Operations manual and runbooks
├── consolidation/    - Module consolidation progress
├── security/         - Security hardening documentation
├── performance/      - Performance testing results
├── review/           - Final review and sign-offs
├── assessment/       - Honest assessments and gap analysis
└── other/            - Miscellaneous documentation
```

---

## 📝 Adding New Documentation

### Step 1: Determine Category

Ask yourself: "Which category does this belong to?"

- **Architecture**: System design, integration points, data flow
- **Deployment**: Deployment procedures, checklists, environment setup
- **Operations**: Daily operations, monitoring, troubleshooting
- **Consolidation**: Phase completion reports, consolidation strategies
- **Security**: Security hardening, authentication, encryption
- **Performance**: Performance testing, benchmarking, optimization
- **Review**: Final reviews, sign-offs, readiness assessments
- **Assessment**: Honest assessments, gap analysis, roadmaps

### Step 2: Create File with Descriptive Name

```bash
# Good examples:
phase_10_critical_integration_complete.md
deployment_production_runbook.md
operations_daily_checklist.md
security_authentication_guide.md

# Bad examples:
new_doc.md
notes.md
stuff.md
```

### Step 3: Add to Appropriate Directory

```bash
# Create file in appropriate directory
New-Item -Path "D:\Aaroneous\docs\consolidation\phase_10_critical_integration_complete.md" -ItemType File

# Or use Write command
Write-Content -Path "D:\Aaroneous\docs\consolidation\phase_10_critical_integration_complete.md" -Value $content
```

### Step 4: Update Documentation Index (Optional)

For major additions, update `docs/INDEX.md`:

```markdown
## New Section: Phase 10 Completion

| File | Status | Description |
|------|--------|-------------|
| phase_10_critical_integration_complete.md | ✅ Complete | Critical integration completion report |
```

---

## 📊 Documentation Statistics

Track documentation progress:

```powershell
# Count total files
Get-ChildItem -Path "D:\Aaroneous\docs" -Filter "*.md" -Recurse | Measure-Object | Select-Object -ExpandProperty Count

# Count by category
Get-ChildItem -Path "D:\Aaroneous\docs" -Filter "*.md" -Recurse | Group-Object { Split-Path -Parent $_.FullName } | Format-Table Name, Count
```

---

## 🔍 Finding Documentation

### Quick Navigation

1. **Project Overview**: `README.md` (root)
2. **Complete Index**: `docs/INDEX.md`
3. **Current Work**: `TODO.md` (root)
4. **Agent Tasks**: `AGENTS.md` (root)
5. **Status Summary**: `PROJECT_STATUS_SUMMARY.md`

### By Category

- **Architecture**: `docs/architecture/`
- **Deployment**: `docs/deployment/`
- **Operations**: `docs/operations/`
- **Consolidation**: `docs/consolidation/`
- **Security**: `docs/security/`
- **Performance**: `docs/performance/`
- **Review**: `docs/review/`
- **Assessment**: `docs/assessment/`

## Build Stabilization Notes (Phase X)

- **Dependency Resolution**: `zstd-sys` on Windows is highly sensitive to `pkg-config`. If builds fail due to "Can't probe for zstd", ensure `PKG_CONFIG = 'false'` and `ZSTD_NO_PKG_CONFIG = '1'` are set in `.cargo/config.toml`. 
- **Header Paths**: Native dependencies (like `rocksdb` and `zstd`) require explicit MSVC header paths in `BINDGEN_EXTRA_CLANG_ARGS` to resolve missing `stddef.h` errors.
- **Mutex Management**: Async locking across `await` points in `spatial_kinetic.rs` must be managed by scoping the `MutexGuard` rather than holding it across the await point.

---

### When to Clean Up

- **Regular audits**: Every 2 weeks
- **Before major releases**: Ensure all docs are current
- **When adding new sections**: Review existing docs for duplicates

### What to Keep

- ✅ All progress documentation - Never delete
- ✅ Completed phase reports
- ✅ Assessment and gap analysis
- ✅ Lessons learned documents

### What Can Be Archived

- ⏳ Drafts that were never finalized (move to `docs/other/drafts/`)
- ⏳ Outdated versions (keep latest, archive old)
- ⏳ Duplicate content (consolidate into single source)

---

## 📝 Documentation Quality Checklist

Before adding new documentation:

- [ ] **Descriptive Name**: Does the filename clearly describe content?
- [ ] **Appropriate Category**: Is it in the right docs/ subdirectory?
- [ ] **Status Indicators**: Are completion status markers included?
- [ ] **Relevant Metadata**: Date, author, version included if applicable?
- [ ] **No Duplicates**: Does this duplicate existing documentation?
- [ ] **Index Updated**: Is docs/INDEX.md updated for major additions?

---

## 🔄 Documentation Workflow

### For Phase Completion:

1. **Execute Phase Work** → Create completion report
2. **Place in Consolidation Directory** → `docs/consolidation/phase_X_*.md`
3. **Update Index** → Add to `docs/INDEX.md`
4. **Archive Old Phases** → Move completed phases to archive

### For Ongoing Work:

1. **Create Draft** → Place in `docs/other/drafts/`
2. **Complete and Review** → Move to appropriate category
3. **Update Index** → Add to `docs/INDEX.md`
4. **Maintain Root Files** → Keep README, INDEX, AGENTS, TODO current

---

## 📊 Monitoring Documentation Health

### Regular Checks:

```powershell
# Check for orphaned files in root (should only be 4 key files)
Get-ChildItem -Path "D:\Aaroneous" -Filter "*.md" | Where-Object { $_.FullName -notmatch "^D:\\Aaroneous\\(README|INDEX|AGENTS|TODO|PROJECT_STATUS_SUMMARY)" }

# Check docs directory structure
Get-ChildItem -Path "D:\Aaroneous\docs" -Directory | Format-Table Name, @{Label="Files"; Expression={ (Get-ChildItem $_.FullName -Filter "*.md").Count }}
```

### Documentation Health Metrics:

- **Total Files**: Track growth over time
- **Coverage**: Ensure all categories have content
- **Recency**: Check last modified dates
- **Completeness**: Verify phase reports are complete

---

## 🎯 Best Practices

### Do ✅

- Add new documentation to appropriate `docs/` subdirectory
- Use descriptive, consistent naming conventions
- Include status indicators (✅ Complete, 🟡 In Progress, ⏳ Pending)
- Update indexes for major additions
- Keep root directory clean (only 4 key files)
- Archive old phases when complete

### Don't ❌

- Delete existing progress documentation
- Add files directly to root directory (except 4 key files)
- Use vague names like "notes.md" or "temp.md"
- Duplicate content across multiple files
- Forget to update documentation index

---

## 📞 Support

For questions about documentation organization:
1. Check `docs/CLEANUP_SUMMARY.md` for cleanup guidelines
2. Review `docs/MAINTENANCE_GUIDE.md` (this file)
3. Consult with team lead on categorization decisions

---

*Last Updated: Week 6, Day 13 | Status: ✅ MAINTENANCE GUIDE COMPLETE*

