# Genetic Harvesting Action Plan

## Executive Summary

You are about to harvest genetic material from 5 carefully selected GGUF models, extracting 25,000 total genetic loci that will form the foundation of your 7 specialist agents.

**Timeline**: 4.5-8 hours (depending on parallelization)  
**Models**: 5 × 80-30B parameter range  
**Output**: 7 genetically-informed specialists ready for Phase 4 event loop

---

## STEP 1: Verify Setup (5 minutes)

### 1.1 Check Genetics Folder Structure
```powershell
# Verify folders exist
Get-ChildItem "D:\Aaroneous\genetics" -Recurse
```

Expected output:
```
D:\Aaroneous\genetics\
├── gguf_sources/
├── extracted_profiles/
├── specialist_genetic_profiles/
└── breeding_history/
```

### 1.2 Verify GGUF Models in LM Studio
```powershell
# Confirm models are in LM Studio
Get-ChildItem "$env:USERPROFILE\.lmstudio\models" -Recurse -Filter "*.gguf" | 
  Where-Object {!$_.Name.StartsWith("mmproj")} | 
  Select-Object Name, @{N="SizeGB";E={[math]::Round($_.Length/1GB,2)}} |
  Sort-Object SizeGB -Descending |
  Select-Object -First 5
```

Expected models:
- ✓ Qwen3-Next-80B (45.16 GB)
- ✓ Hermes-4-70B (39.6 GB)
- ✓ Qwen3-Coder-30B (17.35 GB)
- ✓ GLM-4.7-Flash (16.89 GB)
- ✓ Gemma-3-27B (14.5 GB)

### 1.3 Verify Rust Build
```bash
cd D:\Aaroneous
cargo build --lib --release
```

Expected: Build succeeds, genetics module compiles

---

## STEP 2: Import GGUF Models (10 minutes)

### 2.1 Run Import Script
```powershell
cd D:\Aaroneous
powershell -ExecutionPolicy Bypass -File scripts\import_gguf_models.ps1 -Verify
```

This script will:
- ✓ Find each model in LM Studio
- ✓ Copy to `D:\Aaroneous\genetics\gguf_sources\`
- ✓ Verify checksums (if -Verify flag)
- ✓ Report size and import time

### 2.2 Monitor Progress
Expected output:
```
📁 Source: C:\Users\aarog\.lmstudio\models
📁 Destination: D:\Aaroneous\genetics\gguf_sources

🔍 Searching for: Qwen3-Next-80B
✓ FOUND: Qwen3-Next-80B-A3B-Instruct-Q4_K_M.gguf
  Size: 45.16 GB
📥 Copying...
✅ SUCCESS: Copied (speed displayed)

[... repeat for other 4 models ...]

✅ Successful: 5/5
📊 Total Size Imported: 133.5 GB
🎉 ALL MODELS IMPORTED SUCCESSFULLY!
```

### 2.3 Verify Imports
```powershell
# Check all 5 models copied successfully
Get-ChildItem "D:\Aaroneous\genetics\gguf_sources" -Filter "*.gguf" | 
  Select-Object Name, @{N="SizeGB";E={[math]::Round($_.Length/1GB,2)}}
```

Expected: 5 files totaling ~133.5 GB

---

## STEP 3: Prepare Extraction Environment (5 minutes)

### 3.1 Create Extraction Logs Directory
```powershell
New-Item -ItemType Directory -Path "D:\Aaroneous\genetics\extraction_logs" -Force
```

### 3.2 Verify Pipeline Configuration
```bash
# Check extraction_pipeline.json is valid
jq . D:\Aaroneous\config\extraction_pipeline.json > nul
echo "Pipeline config valid"
```

### 3.3 Create Progress Tracking Script
This script will be used during extraction to monitor progress:
```powershell
# Create file: D:\Aaroneous\scripts\monitor_extraction.ps1
# (See MONITORING section below for full script)
```

---

## STEP 4: Start Genetic Extraction (4-8 hours)

### 4.1 Option A: Sequential Extraction (Simpler, ~7 hours)

For sequential extraction, run models one at a time:

```bash
cd D:\Aaroneous

# Model 1: Qwen3-Next-80B (120-150 min)
cargo run --release --example extract_genetics \
  --model-path genetics/gguf_sources/qwen3-next-80b.gguf \
  --output genetics/extracted_profiles/qwen3-next-80b.genetics.json \
  --log-level info 2>&1 | tee genetics/extraction_logs/qwen3-80b.log

# Model 2: Hermes-4-70B (110-140 min)
cargo run --release --example extract_genetics \
  --model-path genetics/gguf_sources/hermes-4-70b.gguf \
  --output genetics/extracted_profiles/hermes-4-70b.genetics.json \
  --log-level info 2>&1 | tee genetics/extraction_logs/hermes-70b.log

# Model 3: Qwen3-Coder-30B (45-60 min)
cargo run --release --example extract_genetics \
  --model-path genetics/gguf_sources/qwen3-coder-30b.gguf \
  --output genetics/extracted_profiles/qwen3-coder-30b.genetics.json \
  --log-level info 2>&1 | tee genetics/extraction_logs/qwen3-coder.log

# Model 4: GLM-4.7-Flash (45-60 min)
cargo run --release --example extract_genetics \
  --model-path genetics/gguf_sources/glm-4.7-flash.gguf \
  --output genetics/extracted_profiles/glm-4.7-flash.genetics.json \
  --log-level info 2>&1 | tee genetics/extraction_logs/glm-47.log

# Model 5: Gemma-3-27B (50-70 min)
cargo run --release --example extract_genetics \
  --model-path genetics/gguf_sources/gemma-3-27b.gguf \
  --output genetics/extracted_profiles/gemma-3-27b.genetics.json \
  --log-level info 2>&1 | tee genetics/extraction_logs/gemma-3.log
```

### 4.2 Option B: Parallel Extraction (Faster, ~2.5 hours with 4 GPU)

If you have 4+ GPU cores or want parallel CPU extraction:

```bash
# Create background jobs for each extraction
$job1 = Start-Job -ScriptBlock {
  cd D:\Aaroneous
  cargo run --release --example extract_genetics `
    --model-path genetics/gguf_sources/qwen3-next-80b.gguf `
    --output genetics/extracted_profiles/qwen3-next-80b.genetics.json
}

$job2 = Start-Job -ScriptBlock {
  cd D:\Aaroneous
  cargo run --release --example extract_genetics `
    --model-path genetics/gguf_sources/hermes-4-70b.gguf `
    --output genetics/extracted_profiles/hermes-4-70b.genetics.json
}

# ... repeat for other 3 models

# Monitor all jobs
Get-Job | Wait-Job
Get-Job | Receive-Job
```

### 4.3 Extraction Progress Indicators

**Stage 1: Structural Analysis** (8-15 min per model)
```
[████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 20%
Analyzing attention heads... 1200 loci
Analyzing layers... 800 loci extracted
Mapping embeddings... 600 loci extracted
Identifying biases... 400 loci extracted
```

**Stage 2: Behavioral Profiling** (25-60 min per model)
```
[████████████████████░░░░░░░░░░░░░░░░░░░░░░] 45%
Running test suite... 125/400 tests complete
- Reasoning domain: 5/50 tests
- Knowledge domain: 10/50 tests
- Code domain: 0/50 tests (pending)
Extracting 2000 behavioral loci...
```

**Stage 3: DAG/RAG Analysis** (15-45 min per model)
```
[████████████████████████████░░░░░░░░░░░░░░░] 70%
Analyzing task decomposition...
Measuring context relevance...
Studying synthesis patterns...
Extracting 1500 DAG/RAG loci...
```

**Stage 4: Encoding** (4-15 min per model)
```
[████████████████████████████████████████░░░] 95%
Normalizing 5000 loci to [0.0, 1.0]
Initializing epigenetic state
Calculating genetic distances
Validating quality gates
```

### 4.4 Troubleshooting Extraction

**If extraction stalls:**
- Monitor memory: `Get-Process | Sort-Object PM -Descending | Select-Object -First 5`
- Monitor disk space: `Get-Volume | Select-Object DriveLetter, Size, SizeRemaining`
- Check logs: `Get-Content D:\Aaroneous\genetics\extraction_logs\*.log -Tail 20`

**If a model fails:**
- Delete incomplete profile: `Remove-Item D:\Aaroneous\genetics\extracted_profiles\*.partial`
- Retry that model individually
- Check available RAM (need 12-16 GB peak)

**If out of disk space:**
- Extraction creates ~3MB per profile
- Total needed: ~20 GB working space
- Free up space or pause other processes

---

## STEP 5: Validate Extracted Profiles (30 minutes)

### 5.1 Check Profile Integrity
```bash
# Verify all profiles exist
Get-ChildItem "D:\Aaroneous\genetics\extracted_profiles" -Filter "*.json"
```

Expected output:
```
qwen3-next-80b.genetics.json       (3 MB)
hermes-4-70b.genetics.json         (3 MB)
qwen3-coder-30b.genetics.json      (3 MB)
glm-4.7-flash.genetics.json        (3 MB)
gemma-3-27b.genetics.json          (3 MB)
```

### 5.2 Validate Profile Structure
```bash
# Check first profile is valid JSON and has 5000 loci
$profile = Get-Content "D:\Aaroneous\genetics\extracted_profiles\qwen3-next-80b.genetics.json" | ConvertFrom-Json
Write-Host "Loci count: $($profile.genetic_loci.Count)"
Write-Host "Categories: $(($profile.genetic_loci | Select -ExpandProperty category -Unique).Count)"
Write-Host "Valid: $(($profile.genetic_loci | Where {$_.value -ge 0 -and $_.value -le 1}).Count) / $($profile.genetic_loci.Count)"
```

Expected output:
```
Loci count: 5000
Categories: 8
Valid: 5000 / 5000
```

### 5.3 Check Genetic Diversity
```bash
# Calculate genetic distances between models
cargo run --release --example compare_genetics \
  --profile1 genetics/extracted_profiles/qwen3-next-80b.genetics.json \
  --profile2 genetics/extracted_profiles/hermes-4-70b.genetics.json \
  --all-pairs
```

Expected output (genetic distances > 0.30 = good diversity):
```
Qwen3-80B vs Hermes-70B: 0.35 ✓
Qwen3-80B vs Qwen3-Coder: 0.28 ✓
Hermes-70B vs GLM-4.7: 0.42 ✓
GLM-4.7 vs Gemma-3: 0.38 ✓
```

### 5.4 Generate Analysis Report
```bash
# Create summary report
cargo run --release --example analyze_genetics \
  --input-dir genetics/extracted_profiles \
  --output genetics/analysis_reports/extraction_summary.md
```

This generates:
- ✓ Summary statistics per model
- ✓ Genetic distance matrix
- ✓ Category distribution analysis
- ✓ Specialization markers identified
- ✓ Breeding recommendations

---

## STEP 6: Create Specialist Genetic Profiles (30 minutes)

### 6.1 Assign Base Genetics to Specialists
```bash
cd D:\Aaroneous

# Odin: Primary = Qwen3-80B, Secondary = Hermes-70B
cargo run --release --example assign_specialist \
  --specialist odin \
  --primary genetics/extracted_profiles/qwen3-next-80b.genetics.json \
  --secondary genetics/extracted_profiles/hermes-4-70b.genetics.json \
  --output genetics/specialist_genetic_profiles/odin.genetics.json

# Merlin: Primary = Hermes-70B, Secondary = Qwen3-80B
cargo run --release --example assign_specialist \
  --specialist merlin \
  --primary genetics/extracted_profiles/hermes-4-70b.genetics.json \
  --secondary genetics/extracted_profiles/qwen3-next-80b.genetics.json \
  --output genetics/specialist_genetic_profiles/merlin.genetics.json

# Hephaestus: Primary = Qwen3-Coder, Secondary = GLM-4.7
cargo run --release --example assign_specialist \
  --specialist hephaestus \
  --primary genetics/extracted_profiles/qwen3-coder-30b.genetics.json \
  --secondary genetics/extracted_profiles/glm-4.7-flash.genetics.json \
  --output genetics/specialist_genetic_profiles/hephaestus.genetics.json

# Ariel: Primary = Gemma-3, Secondary = Hermes-70B
cargo run --release --example assign_specialist \
  --specialist ariel \
  --primary genetics/extracted_profiles/gemma-3-27b.genetics.json \
  --secondary genetics/extracted_profiles/hermes-4-70b.genetics.json \
  --output genetics/specialist_genetic_profiles/ariel.genetics.json

# Argus: Primary = Hermes-70B, Secondary = Qwen3-Coder
cargo run --release --example assign_specialist \
  --specialist argus \
  --primary genetics/extracted_profiles/hermes-4-70b.genetics.json \
  --secondary genetics/extracted_profiles/qwen3-coder-30b.genetics.json \
  --output genetics/specialist_genetic_profiles/argus.genetics.json

# Dionysus: Primary = Gemma-3, Secondary = Qwen3-80B
cargo run --release --example assign_specialist \
  --specialist dionysus \
  --primary genetics/extracted_profiles/gemma-3-27b.genetics.json \
  --secondary genetics/extracted_profiles/qwen3-next-80b.genetics.json \
  --output genetics/specialist_genetic_profiles/dionysus.genetics.json

# Omni: Hybrid of Qwen3-80B and Hermes-70B (50/50 blend)
cargo run --release --example breed_specialist \
  --parent1 genetics/extracted_profiles/qwen3-next-80b.genetics.json \
  --parent2 genetics/extracted_profiles/hermes-4-70b.genetics.json \
  --weight1 0.5 \
  --weight2 0.5 \
  --specialist omni \
  --output genetics/specialist_genetic_profiles/omni.genetics.json
```

### 6.2 Verify Specialist Profiles
```bash
# Check all 7 specialist profiles created
Get-ChildItem "D:\Aaroneous\genetics\specialist_genetic_profiles" -Filter "*.json" | 
  Select-Object Name, @{N="SizeKB";E={[math]::Round($_.Length/1KB,1)}}
```

Expected output:
```
odin.genetics.json           (3.1 KB)
merlin.genetics.json         (3.1 KB)
hephaestus.genetics.json     (3.1 KB)
ariel.genetics.json          (3.1 KB)
argus.genetics.json          (3.1 KB)
dionysus.genetics.json       (3.1 KB)
omni.genetics.json           (3.1 KB)
```

---

## STEP 7: Validate DAG/RAG Genetic Patterns (30 minutes)

### 7.1 Verify DAG Genetics
```bash
# Check that DAG genes align with specialist roles
cargo run --release --example validate_dag_genetics \
  --profile genetics/specialist_genetic_profiles/odin.genetics.json \
  --expected-trait "strategic_vision_focus" \
  --expected-value-range "0.85,1.0"
```

Expected: Odin should have high DAG decomposition genes

### 7.2 Verify RAG Genetics
```bash
# Check that RAG genes align with specialist roles
cargo run --release --example validate_rag_genetics \
  --profile genetics/specialist_genetic_profiles/merlin.genetics.json \
  --expected-trait "pattern_recognition_sensitivity" \
  --expected-value-range "0.85,1.0"
```

Expected: Merlin should have high pattern recognition genes

### 7.3 Generate Validation Report
```bash
cargo run --release --example validate_genetics \
  --specialist-dir genetics/specialist_genetic_profiles \
  --output genetics/analysis_reports/validation_report.md
```

---

## STEP 8: Integration & Next Phase (Ready Check)

### 8.1 Create Integration Manifest
```bash
# Generate manifest showing what's ready for Phase 4
cat > D:\Aaroneous\genetics\GENETICS_READY.json << 'EOF'
{
  "extraction_date": "$(Get-Date -Format 'yyyy-MM-ddTHH:mm:ssZ')",
  "models_extracted": 5,
  "total_loci": 25000,
  "specialists_created": 7,
  "status": "READY_FOR_PHASE_4",
  "specialists": {
    "odin": {"status": "ready", "genetic_profile": "specialist_genetic_profiles/odin.genetics.json"},
    "merlin": {"status": "ready", "genetic_profile": "specialist_genetic_profiles/merlin.genetics.json"},
    "hephaestus": {"status": "ready", "genetic_profile": "specialist_genetic_profiles/hephaestus.genetics.json"},
    "ariel": {"status": "ready", "genetic_profile": "specialist_genetic_profiles/ariel.genetics.json"},
    "argus": {"status": "ready", "genetic_profile": "specialist_genetic_profiles/argus.genetics.json"},
    "dionysus": {"status": "ready", "genetic_profile": "specialist_genetic_profiles/dionysus.genetics.json"},
    "omni": {"status": "ready", "genetic_profile": "specialist_genetic_profiles/omni.genetics.json"}
  }
}
EOF
```

### 8.2 Create Integration Guide
```bash
# Create guide for using genetic profiles in Phase 4
cat > D:\Aaroneous\PHASE4_GENETIC_INTEGRATION.md << 'EOF'
# Phase 4: Event Loop Integration with Genetic Profiles

## Loading Specialist Genetics

When creating specialists in Phase 4:

\`\`\`rust
use aaroneous::genetics::SpecialistGenome;
use std::fs;

// Load Odin's genetic profile
let odin_genetics = fs::read_to_string("genetics/specialist_genetic_profiles/odin.genetics.json")?;
let odin_genome: SpecialistGenome = serde_json::from_str(&odin_genetics)?;

// Create specialist with genetics
let odin = create_specialist_with_genetics(
    "odin",
    "strategic_planner",
    odin_genome
);
\`\`\`

## Using Genetics in DAG/RAG

Specialists use their genetic profiles to:

1. **Influence DAG decomposition depth**
   - Odin: 0.87 → Deep task decomposition
   - Hephaestus: 0.79 → Practical, direct approach

2. **Guide RAG retrieval patterns**
   - Merlin: 0.92 → Sophisticated synthesis
   - Hephaestus: 0.68 → Direct, relevant retrieval

3. **Epigenetic adaptation during training**
   - Success in tasks → reduce methylation of relevant genes
   - Federation feedback → adjust histone modifications

## Monitoring Genetic Expression

During event loop:

\`\`\`rust
// Check current epigenetic state
let expressed_value = specialist.genome
    .expressed_trait_value(&dag_decomposition_locus);

// Adjust based on DAG success
if dag_success {
    specialist.genome.epigenetic_state.methylation = 0.1;
}
\`\`\`

See D:\Aaroneous\SPECIALIST_GENETICS_GUIDE.md for full details.
EOF
```

### 8.3 Readiness Checklist

- [ ] All 5 GGUF models imported (133.5 GB)
- [ ] All 5 models extracted (25,000 loci total)
- [ ] All profiles validate (5000 loci each, 8 categories)
- [ ] Genetic diversity confirmed (distances > 0.30)
- [ ] All 7 specialists have genetic profiles
- [ ] DAG genetics validated
- [ ] RAG genetics validated
- [ ] Integration manifest created
- [ ] Phase 4 integration guide created

---

## MONITORING & DIAGNOSTICS

### Watch Extraction Progress
```bash
# In separate terminal, monitor logs in real-time
Get-Content D:\Aaroneous\genetics\extraction_logs\*.log -Wait
```

### Check System Resources
```powershell
# Monitor in real-time during extraction
Get-Process -Name cargo | Select-Object Name, ID, PM, CPU | Format-Table -AutoSize -Refresh

# Check disk space
Get-Volume | Select-Object DriveLetter, Size, SizeRemaining, PercentFree
```

### Validate Extraction Quality
```bash
# After each model completes
cargo run --release --example validate_genetics \
  --profile genetics/extracted_profiles/[model].genetics.json \
  --verbose
```

---

## SUCCESS CRITERIA

### Extraction Complete ✓
- All 5 genetic profiles extracted
- Each has exactly 5,000 loci
- All loci values in [0.0, 1.0]
- All 8 categories represented

### Integration Complete ✓
- All 7 specialists have genetic profiles
- DAG/RAG genetics validated
- Breeding operations successful
- Specialist profiles ready for Phase 4

### System Ready ✓
- Genetic profiles stored and indexed
- Integration manifest created
- Phase 4 guidance documented
- No errors in validation checks

---

## IMMEDIATE NEXT STEP

Once this genetic harvesting is complete:

**Phase 4: Event Loop Implementation**
- Build specialist execution loop
- Integrate genetic expression into DAG/RAG
- Implement enzyme invocation with genetic guidance
- NATS reporting with genetic metrics

All 7 specialists will be genetically informed and ready to begin their evolutionary journey!

---

**Ready to begin genetic harvesting? Start with STEP 1 above.**
