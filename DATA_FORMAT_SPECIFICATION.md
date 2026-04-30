# Aaroneous Data Format Specification
## Complete Guide to Data Ingestion and Format Support

---

## 📋 Table of Contents

1. [Supported Formats](#supported-formats)
2. [Format Specifications](#format-specifications)
3. [Classification Rules](#classification-rules)
4. [Specialist Routing](#specialist-routing)
5. [Data Quality Scoring](#data-quality-scoring)
6. [Error Handling](#error-handling)
7. [Examples](#examples)

---

## Supported Formats

### Model Files

| Format | Extension | Size Typical | Specialist | XP |
|--------|-----------|-------------|------------|-----|
| GGUF | `.gguf` | 500MB-30GB | Merlin | 500-1000 |
| SafeTensors | `.safetensors` | 100MB-10GB | Merlin | 400-800 |
| PyTorch | `.pt`, `.pth` | 50MB-5GB | Hephaestus | 300-600 |
| TensorFlow | `.pb`, `.h5` | 50MB-5GB | Hephaestus | 300-600 |
| ONNX | `.onnx` | 10MB-500MB | Hephaestus | 200-400 |

### Data Files

| Format | Extension | Use Case | Specialist | XP |
|--------|-----------|----------|------------|-----|
| CSV | `.csv` | Tabular data | Ariel | 50-150 |
| JSON | `.json` | Structured data | Ariel | 75-200 |
| Parquet | `.parquet` | Columnar data | Circe | 100-300 |
| Excel | `.xlsx`, `.xls` | Spreadsheets | Ariel | 75-250 |
| TSV | `.tsv` | Tab-separated values | Ariel | 50-150 |
| Avro | `.avro` | Serialized data | Circe | 100-250 |

### Document Files

| Format | Extension | Use Case | Specialist | XP |
|--------|-----------|----------|------------|-----|
| Markdown | `.md` | Documentation | Merlin | 25-100 |
| Text | `.txt` | Text content | Merlin | 25-50 |
| PDF | `.pdf` | Documents | Merlin | 50-200 |
| HTML | `.html` | Web content | Ariel | 50-150 |
| XML | `.xml` | Markup data | Ariel | 50-150 |

### Configuration Files

| Format | Extension | Use Case | Specialist | XP |
|--------|-----------|----------|------------|-----|
| YAML | `.yaml`, `.yml` | Configuration | Odin | 25-75 |
| TOML | `.toml` | Configuration | Odin | 25-75 |
| JSON | `.json` | Configuration | Odin | 25-75 |
| INI | `.ini` | Configuration | Odin | 25-75 |

### Log Files

| Format | Extension | Use Case | Specialist | XP |
|--------|-----------|----------|------------|-----|
| Log | `.log` | System logs | Argus | 25-100 |
| Text | `.txt` | Text logs | Argus | 25-100 |

### Archive Files

| Format | Extension | Contents | Specialist | XP |
|--------|-----------|----------|------------|-----|
| ZIP | `.zip` | Multiple files | Hephaestus | 50-200 |
| TAR | `.tar` | Archive | Hephaestus | 50-200 |
| GZIP | `.gz`, `.tar.gz` | Compressed | Hephaestus | 50-200 |

---

## Format Specifications

### CSV (Comma-Separated Values)

**File Extension:** `.csv`

**Basic Structure:**
```csv
name,age,role,experience
Alice,28,Developer,5
Bob,32,Designer,8
Charlie,25,Analyst,2
```

**Requirements:**
- Header row with column names
- Comma-separated values
- Text values in quotes if they contain commas
- One record per line

**Best Practices:**
```csv
"Last Name","First Name","Email","Department"
"Smith","Alice","alice@company.com","Engineering"
"Johnson","Bob","bob@company.com","Design"
```

**Size Limits:**
- Minimum: 10 bytes (headers only)
- Maximum: 500MB (will be processed)
- Recommended: <100MB for optimal performance

**Quality Scoring:**
```
Perfect CSV:              100 points
Well-formed:              80-99 points
Minor issues (quotes):    60-79 points
Malformed:                40-59 points
Cannot parse:             <40 points
```

---

### JSON (JavaScript Object Notation)

**File Extension:** `.json`

**Basic Structure:**
```json
{
  "specialists": [
    {
      "name": "Merlin",
      "rank": 3,
      "xp": 3200,
      "skills": ["DAG", "RAG"]
    },
    {
      "name": "Ariel",
      "rank": 2,
      "xp": 2500,
      "skills": ["RAG", "API"]
    }
  ]
}
```

**Requirements:**
- Valid JSON syntax
- Proper nesting of objects and arrays
- String values in double quotes
- No trailing commas

**Supported Structures:**
- Objects (key-value pairs)
- Arrays (lists)
- Nested structures
- Mixed types (string, number, boolean, null)

**Best Practices:**
```json
{
  "metadata": {
    "version": "1.0",
    "created": "2024-01-29T12:00:00Z"
  },
  "data": [
    {
      "id": 1,
      "value": "example",
      "enabled": true
    }
  ]
}
```

**Size Limits:**
- Minimum: 2 bytes (`{}`)
- Maximum: 500MB
- Recommended: <100MB for optimal performance

**Quality Scoring:**
```
Valid JSON:               100 points
Parseable with warnings:  75-99 points
Requires normalization:   50-74 points
Invalid JSON:             <50 points
```

---

### Parquet (Apache Parquet)

**File Extension:** `.parquet`

**Description:**
Columnar storage format optimized for large datasets. Excellent for analytics.

**Characteristics:**
- Compressed by default
- Schema-based structure
- Very efficient for querying subsets of columns
- Larger files decompress to massive datasets

**Best for:**
- Data science workflows
- Analytics pipelines
- Time-series data
- High-volume ingestion

**Quality Scoring:**
```
Valid Parquet file:       100 points
Readable schema:          80-99 points
Requires conversion:      60-79 points
Corrupted file:           <60 points
```

---

### GGUF (GPT Generated Unified Format)

**File Extension:** `.gguf`

**Description:**
Modern model format storing weights, embeddings, and metadata.

**Structure:**
```
GGUF Header
├── Magic number
├── Version
├── Tensor count
├── Metadata
└── Tensor data
    ├── Weights
    ├── Embeddings
    └── Configuration
```

**Key Features:**
- Single file format (no distributed weights)
- Complete model in one file
- Includes metadata and config
- Optimized for inference

**Size Considerations:**
- 7B model: ~4GB
- 13B model: ~8GB
- 70B model: ~40GB

**Quality Scoring:**
```
Valid GGUF:               100 points
Readable tensors:        80-99 points
Incomplete weights:      60-79 points
Corrupted file:          <60 points
Cannot load:             <50 points
```

---

### YAML Configuration Files

**File Extension:** `.yaml`, `.yml`

**Basic Structure:**
```yaml
database:
  host: localhost
  port: 5432
  credentials:
    username: admin
    password: secret

features:
  - ingestion
  - analytics
  - dashboard

settings:
  timeout: 30
  retries: 3
  enabled: true
```

**Requirements:**
- Proper indentation (2 or 4 spaces)
- Colon after keys
- Dash for list items
- No tab characters

**Key Features:**
- Human-readable
- Hierarchical structure
- Native types (string, number, boolean, null)
- Comments with `#`

**Quality Scoring:**
```
Valid YAML:               100 points
Parseable YAML:          80-99 points
Requires formatting:     60-79 points
Invalid syntax:          <60 points
```

---

### Archive Files (ZIP, TAR, GZIP)

**File Extensions:** `.zip`, `.tar`, `.gz`, `.tar.gz`

**Handling:**
When archive files are detected, the system:
1. Extracts contents
2. Analyzes each contained file
3. Routes files to appropriate specialists
4. Records extraction metadata
5. Stores original archive

**Best Practices:**
```
project.zip
├── data.csv
├── model.gguf
├── config.yaml
└── README.md
```

All files are processed individually, with parent-file tracking.

**Size Limits:**
- Uncompressed size must be <2GB
- Individual extracted files follow format limits
- Deeply nested archives may be rejected

---

## Classification Rules

### File Type Detection

**Priority Order:**
1. **File Extension** - Primary classifier
2. **MIME Type** - Secondary classifier
3. **File Magic Bytes** - Tertiary classifier (for headerless files)
4. **Content Inspection** - Fallback for ambiguous files

### Format Ambiguities

**`.txt` files:**
```
Detection order:
1. Check if valid JSON → classify as JSON
2. Check if valid YAML → classify as YAML
3. Check if valid CSV → classify as CSV
4. Default → Text document
```

**`.json` files:**
```
Always treated as JSON
No fallback classification
Invalid JSON is rejected
```

**Archive Contents:**
```
Each extracted file classified independently
Parent archive type noted in metadata
```

---

## Specialist Routing

### Routing Rules

**Content Analysis:**
```
Is it a model file?
├─ YES → Route to Merlin (Knowledge)
└─ NO
   ├─ Is it tabular/structured data?
   │  ├─ YES → Route to Ariel (UI Designer) or Circe (Analyst)
   │  └─ NO
   │     ├─ Is it configuration?
   │     │  ├─ YES → Route to Odin (Leader)
   │     │  └─ NO
   │     │     ├─ Is it a tool/integration file?
   │     │     │  ├─ YES → Route to Hephaestus (Inventor)
   │     │     │  └─ NO
   │     │     │     ├─ Is it a log file?
   │     │     │     │  ├─ YES → Route to Argus (Guardian)
   │     │     │     │  └─ NO → Route to Merlin (default)
```

### Routing Table

| Content Type | Primary | Secondary | XP Base |
|--------------|---------|-----------|---------|
| Model | Merlin | Hephaestus | 600 |
| Tabular Data | Ariel | Circe | 100 |
| Time-Series | Circe | Merlin | 120 |
| Configuration | Odin | Hephaestus | 50 |
| Code/Script | Hephaestus | Merlin | 75 |
| Log File | Argus | Merlin | 50 |
| Documentation | Merlin | Odin | 50 |
| Unknown | Merlin | - | 25 |

---

## Data Quality Scoring

### Quality Metrics

**File Integrity (40 points max):**
```
✓ Valid file format:      20 points
✓ Complete file:          10 points
✓ Readable structure:     10 points
```

**Content Quality (30 points max):**
```
✓ Well-formed:            10 points
✓ Complete data:          10 points
✓ Normalized format:      10 points
```

**Completeness (20 points max):**
```
✓ No missing values:      10 points
✓ All headers present:    5 points
✓ Consistent schema:      5 points
```

**Documentation (10 points max):**
```
✓ Metadata included:      5 points
✓ Descriptions present:   5 points
```

### XP Multipliers

Based on quality score:
```
90-100 points: 1.5x XP multiplier (Excellent)
80-89 points:  1.2x XP multiplier (Good)
70-79 points:  1.0x XP multiplier (Acceptable)
60-69 points:  0.8x XP multiplier (Fair)
<60 points:    0.5x XP multiplier (Poor)
```

### Example Calculation

```
Base XP: 100
File size quality: CSV 500 lines = 10KB (+5)
Content quality: All fields present = +10
Format quality: Perfect CSV = +10
Metadata: None = 0

Total Quality: 80 points (1.2x multiplier)
Final XP: 100 × 1.2 = 120 XP
```

---

## Error Handling

### Processing Errors

**File Not Readable:**
```
Action: Move to failed/ folder
Notification: Event log entry
Retry: Manual retry available
XP: 0 points
```

**Format Invalid:**
```
Action: Move to failed/ folder
Notification: Event log entry with error details
Retry: Manual retry after fixing
XP: 0 points
```

**File Too Large:**
```
Action: Reject and notify
Max: 500MB per file
Suggestion: Split file or contact admin
XP: 0 points
```

**Permission Denied:**
```
Action: Move to failed/ folder
Notification: Security event log
Investigation: Check file permissions
XP: 0 points
```

### Recovery Options

```bash
# View failed ingestions
aaroneous query ingestions --filter "status=failed"

# Get error details
aaroneous query ingestion --file "myfile.csv" --detailed

# Retry manually
aaroneous ingestion retry --file "myfile.csv"

# Fix and reprocess
# 1. Move file from failed/ to inbox/
# 2. Fix any format issues
# 3. Drop back in inbox/
# 4. System reprocesses
```

---

## Examples

### Example 1: CSV File Processing

**Input File:**
```csv
date,event,severity,specialist
2024-01-29,Data ingestion,INFO,Ariel
2024-01-29,Skill unlock,SUCCESS,Merlin
2024-01-29,Rank up,SUCCESS,Odin
```

**Processing Steps:**
1. Detect: CSV format (extension `.csv`)
2. Parse: 3 data rows + 1 header
3. Validate: All fields present, proper format
4. Score: 85 points (good CSV)
5. Route: Ariel (tabular data specialist)
6. XP: 100 × 1.2 = 120 XP to Ariel
7. Event: "CSV file processed (3 rows, 120 XP)"

**Result:**
```
Status: SUCCESS
Specialist: Ariel
XP Awarded: 120
Quality: Good
Processing Time: 0.2s
```

---

### Example 2: GGUF Model File Processing

**Input File:**
- Filename: `mistral-7b.gguf`
- Size: 4.2 GB
- Format: GGUF

**Processing Steps:**
1. Detect: GGUF format (magic bytes verification)
2. Load: Validate tensor structure
3. Extract: Weights, embeddings, config
4. Score: 95 points (valid GGUF)
5. Route: Merlin (knowledge/model specialist)
6. XP: 800 × 1.25 = 1000 XP to Merlin
7. Event: "Model file processed (Mistral 7B, 1000 XP)"

**Result:**
```
Status: SUCCESS
Specialist: Merlin
XP Awarded: 1000
Model: Mistral 7B
Parameters: 7B
Processing Time: 15s
```

---

### Example 3: Archive File Processing

**Input File:**
- Filename: `project.zip`
- Contents:
  - `data.csv` (100 rows)
  - `model.gguf` (500MB)
  - `config.yaml`

**Processing Steps:**
1. Detect: ZIP archive
2. Extract: 3 files identified
3. Process each file:
   - `data.csv` → Ariel (150 XP)
   - `model.gguf` → Merlin (800 XP)
   - `config.yaml` → Odin (60 XP)
4. Total XP: 1010 XP distributed
5. Event: "Archive processed (3 files, 1010 total XP)"

**Result:**
```
Status: SUCCESS
Files Processed: 3
Total XP: 1010
  - Ariel: 150 XP
  - Merlin: 800 XP
  - Odin: 60 XP
Processing Time: 20s
```

---

### Example 4: Malformed JSON

**Input File:**
```json
{
  "data": [
    {"name": "Alice", "age": 28},
    {"name": "Bob", "age": 32  // Missing closing brace
  ]
}
```

**Processing Steps:**
1. Detect: JSON format (extension `.json`)
2. Parse: Syntax error detected
3. Score: 25 points (invalid)
4. Action: Move to `failed/` folder
5. Event: "JSON file rejected (syntax error at line 4)"
6. XP: 0 points

**Result:**
```
Status: FAILED
Reason: Invalid JSON syntax
Error Line: 4
Suggestion: Fix syntax and retry
File Location: failed/malformed_data.json
XP Awarded: 0
```

**Recovery:**
```bash
# Fix the JSON file
# Reprocess
aaroneous ingestion retry --file "malformed_data.json"
```

---

## FAQ

### Q: What's the maximum file size?
**A:** 500MB per file. Larger files should be split into chunks.

### Q: Can I process password-protected archives?
**A:** Currently no. Unprotect and retry.

### Q: What happens if a file has mixed content?
**A:** The system analyzes content structure and routes to most appropriate specialist.

### Q: How long does processing take?
**A:** Typically 0.1-0.5 seconds. Large models (>5GB) take 5-30 seconds.

### Q: Can I manually adjust XP awards?
**A:** Yes, use `aaroneous specialist award` command.

### Q: What if a file is partially corrupted?
**A:** Rejection and move to failed/. Partial recovery may be possible if corruption is at end of file.

---

**Version:** 1.0  
**Last Updated:** 2024-01-29  
**Format Support:** 25+ formats across 7 categories
