# Aaroneous Team Training Guide
## Complete Training for R&D Team

---

## 🎯 Training Objectives

By the end of this training, team members will be able to:

1. ✅ Start and stop the Aaroneous hive
2. ✅ Navigate the TUI dashboard
3. ✅ Use CLI commands for common tasks
4. ✅ Create and manage specialists
5. ✅ Award XP and track progression
6. ✅ Process data files through the system
7. ✅ Monitor system health and respond to issues
8. ✅ Troubleshoot common problems

---

## 📚 Module 1: System Overview (15 minutes)

### What is Aaroneous?

Aaroneous is a **specialist hive management system** for the R&D team:

- **6 Specialists** with unique capabilities working together
- **Autonomous skill progression** through XP and experience
- **Data ingestion pipeline** that automatically routes work to specialists
- **Real-time dashboard** for monitoring all activity
- **Command-line tools** for operational tasks

### Key Concepts

**Specialists:**
- Named AI agents with personality and capabilities
- Gain XP from completing work
- Progress through 5 ranks (Newly Digested → Transcendent)
- Can create new skills through fusion

**Skills:**
- 5 core types: DAG, RAG, MCP, API, Fusion
- Progress from Level 1 to 20
- Combinations create new fusion skills
- Specialists "awaken" skills at mastery

**Events:**
- Record of all system activity
- Ingestion, skill creation, rank ups, XP awards
- Searchable history for debugging

**Data Ingestion:**
- Drop files in `inbox/` folder
- Automatic format detection
- Routed to best specialist
- XP awarded based on quality

### The 6 Specialists

| Name | Role | Archetype | Strength |
|------|------|-----------|----------|
| **Ariel** | UI/UX | Designer | Visual systems |
| **Merlin** | Knowledge | Scholar | Information synthesis |
| **Odin** | Leadership | Leader | Team coordination |
| **Circe** | Analytics | Analyst | Data interpretation |
| **Hephaestus** | Tools | Inventor | System integration |
| **Argus** | Security | Guardian | Risk detection |

---

## 🖥️ Module 2: TUI Dashboard (20 minutes)

### Starting the Dashboard

```bash
aaroneous start --dashboard tui
```

The dashboard launches with **5 interactive pages**.

### Page 1: Home

**What you see:**
- System health percentage
- Total XP awarded
- Specialist count
- Active events
- Recent achievements

**Why it matters:**
- Quick health check
- Spot problems at a glance
- Verify system is running

**Try this:**
- Navigate here first
- Note the health percentage
- Check total XP

---

### Page 2: Specialists

**What you see:**
- List of 6 specialists
- Current XP for each
- Current rank
- Top 3 skills
- Status (active/idle)

**Why it matters:**
- Track individual progress
- See who's advancing fastest
- Identify specialist performance

**Try this:**
1. Scroll through list with ↑/↓
2. Note highest XP specialist
3. Check who's at Rank 3+
4. Look for newest rank-ups

**Example Output:**
```
[Specialist Roster]

✓ Ariel (Rank 2 - 2,500 XP)
  Skills: RAG Lv12, API Lv8, Fusion-RAG-API Lv5

✓ Merlin (Rank 3 - 3,200 XP)
  Skills: DAG Lv15, RAG Lv18, Fusion-DAG-RAG Lv9

✓ Odin (Rank 2 - 1,900 XP)
  Skills: DAG Lv10, MCP Lv7, API Lv6
```

---

### Page 3: Skill Tree

**What you see:**
- All available skills
- Current skill levels
- Specialist who has each skill
- Skill fusion dependencies
- Skill requirements (XP to unlock)

**Why it matters:**
- Understand skill advancement
- Plan specialist development
- See skill combinations

**Try this:**
1. Look for highest-level skill
2. Find newest fusion skill
3. Check which specialist has most skills
4. Note XP requirements for next skill

**Example Output:**
```
[Skill Tree]

Core Skills:
├── DAG (Decomposition)
│   ├── Merlin: Level 15
│   ├── Odin: Level 10
│   └── Unlock at: 500 XP
├── RAG (Retrieval)
│   ├── Ariel: Level 12
│   ├── Merlin: Level 18
│   └── Unlock at: 300 XP
└── ...

Fusion Skills:
├── DAG+RAG
│   ├── Merlin: Level 9 (Awakened)
│   └── Requirements: DAG Lv10, RAG Lv10
└── ...
```

---

### Page 4: Event Log

**What you see:**
- Recent system events (newest first)
- Event types (XP Award, Skill Created, Rank Up, etc.)
- Associated specialist
- Timestamp
- Event details

**Why it matters:**
- Understand what happened and when
- Debug issues
- Track progression over time

**Try this:**
1. Scroll through recent events
2. Find an XP award event
3. Look for a skill creation
4. Note a rank-up event
5. Check event timestamps

**Example Output:**
```
[Event Log]

2024-01-29 14:32:15 | XP AWARD | Merlin +250 XP (Data Ingestion)
2024-01-29 14:28:43 | SKILL CREATED | Merlin: Fusion-DAG-RAG Lv1
2024-01-29 14:15:22 | RANK UP | Merlin → Rank 3
2024-01-29 14:10:01 | FILE PROCESSED | data.csv (Ariel)
2024-01-29 13:45:30 | DATA INGESTION | Routed to Merlin
```

---

### Page 5: Settings

**What you see:**
- System metrics in real-time
- Health percentage
- Memory usage
- Database size
- Processing statistics
- Configuration view

**Why it matters:**
- Monitor performance
- Spot issues early
- Verify configuration

**Try this:**
1. Check system health (should be >80%)
2. Note memory usage (should be <200MB)
3. View database size
4. Check processing rate
5. Verify all settings are correct

**Navigation Shortcuts:**
| Key | Action |
|-----|--------|
| Tab | Switch to next page |
| Shift+Tab | Switch to previous page |
| ↑/↓ | Scroll current page |
| Enter | Expand/collapse sections |
| Q | Quit dashboard |
| ? | Show help |

---

## 📟 Module 3: CLI Commands (30 minutes)

### Starting the System

```bash
# Start hive with TUI dashboard
aaroneous start --dashboard tui

# Start hive without dashboard (headless)
aaroneous start --headless

# Start with custom database location
aaroneous start --db-path D:\custom\path\hive.db
```

### Checking System Health

```bash
# Quick health check
aaroneous status health

# Example output:
# System Health: 87%
# Uptime: 2h 34m 12s
# Specialists: 6/6 active
# Total Events: 1,247
# Error Count: 2

# Detailed health information
aaroneous status health --detailed

# Continuous monitoring (updates every 5 seconds)
aaroneous status health --watch 5

# JSON output for parsing
aaroneous status health --json
```

**Try this:**
```bash
aaroneous status health
# Take note of the health percentage
# This is your baseline
```

### Viewing Metrics

```bash
# System metrics
aaroneous status metrics

# With resource information
aaroneous status metrics --resources

# Example output:
# Event Processing Rate: 42 events/sec
# Average Event Latency: 15ms
# Memory Usage: 145 MB
# Database Size: 8.3 MB

# Continuous monitoring
aaroneous status metrics --watch 2
```

**Try this:**
```bash
aaroneous status metrics --resources
# Note the current memory usage
# This helps understand baseline
```

---

### Specialist Management

#### List All Specialists

```bash
# Simple list
aaroneous specialist list

# Output:
# 1. Ariel (Rank 2, 2,500 XP)
# 2. Merlin (Rank 3, 3,200 XP)
# 3. Odin (Rank 2, 1,900 XP)
# 4. Circe (Rank 1, 1,600 XP)
# 5. Hephaestus (Rank 1, 1,200 XP)
# 6. Argus (Rank 1, 800 XP)

# Detailed information
aaroneous specialist list --detailed

# Include skill information
aaroneous specialist list --include-skills
```

**Try this:**
```bash
aaroneous specialist list --detailed
# Review each specialist's current state
# Note who has highest XP
```

#### Check Specialist Status

```bash
# Check one specialist
aaroneous specialist status --name "Merlin"

# Example output:
# Name: Merlin
# Archetype: Scholar
# Rank: 3 (Trusted Member)
# XP: 3,200 / 6,000 (to Rank 4)
# Skills:
#   - DAG Level 15
#   - RAG Level 18 (Awakened)
#   - Fusion-DAG-RAG Level 9 (Awakened)
```

#### Award XP to Specialist

```bash
# Award XP to one specialist
aaroneous specialist award \
  --specialist "Merlin" \
  --amount 250 \
  --reason "Data analysis task completed"

# Award to multiple specialists
aaroneous specialist award \
  --specialist "Ariel,Merlin,Odin" \
  --amount 100 \
  --reason "Team achievement"

# Verify award was applied
aaroneous specialist status --name "Merlin"
```

**Try this:**
```bash
# Award 100 XP
aaroneous specialist award --specialist "Ariel" --amount 100 --reason "Testing"

# Check status to confirm
aaroneous specialist status --name "Ariel"
```

#### Create New Specialist (Advanced)

```bash
# Create a new specialist
aaroneous specialist create \
  --name "TestSpecialist" \
  --archetype "Scholar" \
  --initial-xp 500

# Verify creation
aaroneous specialist status --name "TestSpecialist"

# Note: Only use for testing, production has 6 specialists
```

---

### Data Query

#### View System Statistics

```bash
# Quick stats
aaroneous query stats

# Example output:
# Total Specialists: 6
# Total XP Awarded: 12,700
# Average XP per Specialist: 2,117
# Rank Distribution:
#   - Rank 1: 3 specialists
#   - Rank 2: 2 specialists
#   - Rank 3: 1 specialist
# Skills Created: 18
# Ingestion Events: 156

# Detailed statistics
aaroneous query stats --detailed

# JSON format
aaroneous query stats --json
```

**Try this:**
```bash
aaroneous query stats --detailed
# Understand current system state
# Note specialist distribution across ranks
```

#### View Events

```bash
# Recent events (last 50)
aaroneous query events

# Specific number of events
aaroneous query events --limit 100

# Filter by specialist
aaroneous query events --specialist "Merlin" --limit 20

# Filter by event type
aaroneous query events --type "SKILL_CREATED" --limit 10

# Filter by time
aaroneous query events --since "1h" --limit 50

# Example output:
# 2024-01-29 14:32:15 | XP AWARD | Merlin +250
# 2024-01-29 14:28:43 | SKILL CREATED | Merlin: Fusion-DAG-RAG Lv1
# 2024-01-29 14:15:22 | RANK UP | Merlin → Rank 3
```

**Try this:**
```bash
aaroneous query events --limit 20
# Review recent activity
# Get familiar with event format
```

#### View Ingestion History

```bash
# All ingestions
aaroneous query ingestions

# For specific specialist
aaroneous query ingestions --specialist "Merlin"

# Recent ingestions only
aaroneous query ingestions --since "1h"

# Example output:
# File: data.csv | Specialist: Ariel | Status: Processed | XP: 150
# File: model.gguf | Specialist: Merlin | Status: Processing | XP: 0
```

---

### Configuration Management

#### View Configuration

```bash
# Show current configuration
aaroneous config show

# Show all configuration including defaults
aaroneous config show --all

# Example output:
# Inbox Path: D:\Aaroneous\inbox
# Database Path: D:\Aaroneous\hive.db
# Log Level: info
# Dashboard Enabled: true
# File Watch Enabled: true
```

#### Check Configuration Validity

```bash
# Validate current configuration
aaroneous config validate

# Expected output:
# Configuration is valid ✓
# All paths are accessible ✓
# Database is readable ✓
```

**Try this:**
```bash
aaroneous config validate
# Ensure everything is properly configured
```

---

## 📁 Module 4: Data Ingestion (15 minutes)

### How Data Ingestion Works

**Process Flow:**
1. Drop file in `D:\Aaroneous\inbox/`
2. File watcher detects the file
3. Format automatically detected
4. Routed to best specialist
5. XP awarded based on quality
6. File moved to `processed/`

### Supported File Formats

- **Models:** `.gguf`, `.safetensors`, `.pt`, `.pth`
- **Data:** `.csv`, `.json`, `.parquet`, `.xlsx`, `.tsv`
- **Logs:** `.log`, `.txt`
- **Archives:** `.zip`, `.tar`, `.gz`
- **Configuration:** `.yaml`, `.toml`, `.ini`

### Dropping Files

**Method 1: Manual Copy**
```bash
# Copy a file to inbox
Copy-Item data.csv D:\Aaroneous\inbox\

# Wait 10 seconds for processing
Start-Sleep -Seconds 10

# Verify processing
aaroneous query ingestions --limit 1
```

**Method 2: Using CLI**
```bash
# Process file from CLI
aaroneous ingestion process --file "C:\path\to\data.csv"

# Check status
aaroneous query ingestions --limit 1
```

### Verifying Processing

```bash
# Check recent ingestions
aaroneous query ingestions --limit 5

# Check specific specialist's ingestions
aaroneous query ingestions --specialist "Merlin" --limit 10

# View ingestion statistics
aaroneous query stats --detailed | grep -A5 "Ingestion"
```

**Try this:**
```bash
# Create a test file
"test data" | Out-File -FilePath D:\Aaroneous\inbox\test.txt

# Wait for processing
Start-Sleep -Seconds 5

# Check if processed
aaroneous query ingestions --limit 1
```

### Understanding XP Awards

```
File Type       | XP Per File
                |
Small file      | 50-100 XP
Medium file     | 100-200 XP
Large file      | 200-500 XP
Model file      | 500-1000 XP
Complex data    | 100-300 XP (bonus for complexity)
```

---

## 🔧 Module 5: Troubleshooting (20 minutes)

### Common Issues and Solutions

#### Issue: "Command not found"

**Symptom:**
```
'aaroneous' is not recognized as an internal or external command
```

**Solution:**
```bash
# Build the project first
cd D:\Aaroneous
cargo build --release

# Use full path
D:\Aaroneous\target\release\aaroneous.exe status health
```

#### Issue: Dashboard Won't Start

**Symptom:**
```
Error: Failed to initialize terminal
```

**Solution:**
```bash
# Try headless mode
aaroneous start --headless

# Check if database is locked
taskkill /F /IM aaroneous.exe
Start-Sleep -Seconds 5

# Try starting again
aaroneous start --dashboard tui
```

#### Issue: Specialist Not Found

**Symptom:**
```
Error: Specialist "YourName" not found
```

**Solution:**
```bash
# Check correct name
aaroneous specialist list

# Award XP using exact name (case-sensitive)
aaroneous specialist award --specialist "Merlin" --amount 100

# Verify it exists
aaroneous specialist status --name "Merlin"
```

#### Issue: File Not Processing

**Symptom:**
```
File dropped in inbox but doesn't move to processed/
```

**Solution:**
```bash
# Check if file watcher is active
aaroneous status health

# Check supported formats
aaroneous config show | grep -i format

# Try manual ingestion
aaroneous ingestion process --file "D:\Aaroneous\inbox\myfile.csv"

# Check logs for errors
aaroneous query logs --level error --since 1h
```

#### Issue: High Memory Usage

**Symptom:**
```
System Health dropped to 45%
Memory: 600 MB
```

**Solution:**
```bash
# Check current usage
aaroneous status metrics --resources

# If queue backed up, pause ingestion briefly
aaroneous config set inbox.watch false

# Let queue drain
Start-Sleep -Seconds 30

# Resume ingestion
aaroneous config set inbox.watch true

# Verify health improved
aaroneous status health
```

### Getting Help

**Check Documentation:**
```bash
# View help for any command
aaroneous --help
aaroneous specialist --help
aaroneous query --help
aaroneous status --help
```

**Collect Information:**
```bash
# Gather diagnostic bundle
aaroneous support bundle

# This creates a ZIP with:
# - Last 100 log lines
# - System health snapshot
# - Configuration
# - Recent events
```

**Check Logs:**
```bash
# View recent errors
aaroneous query logs --level error --since 1h

# View all logs
aaroneous query logs --limit 100

# Follow logs in real-time
aaroneous query logs --follow
```

---

## 🎓 Module 6: Practical Exercises (30 minutes)

### Exercise 1: Dashboard Navigation (5 min)

**Objective:** Become comfortable navigating the TUI

**Steps:**
1. Start dashboard: `aaroneous start --dashboard tui`
2. View each of 5 pages (use Tab to switch)
3. Note values on each page
4. Use ↑/↓ to scroll within pages
5. Quit with Q

**Success Criteria:**
- Can switch between all 5 pages
- Can scroll within pages
- Can see data on each page

---

### Exercise 2: Specialist Management (5 min)

**Objective:** Learn to manage specialists and award XP

**Steps:**
1. List specialists: `aaroneous specialist list --detailed`
2. Pick one specialist (e.g., "Ariel")
3. Check their status: `aaroneous specialist status --name "Ariel"`
4. Award 150 XP: `aaroneous specialist award --specialist "Ariel" --amount 150 --reason "Training"`
5. Check status again to verify XP increased

**Success Criteria:**
- Can list specialists
- Can check individual status
- Can award XP
- XP increases are reflected

---

### Exercise 3: Data Ingestion (5 min)

**Objective:** Process a file through the system

**Steps:**
1. Create test file:
   ```bash
   "test,data,here" | Out-File D:\Aaroneous\inbox\test_data.csv
   ```
2. Wait 10 seconds for processing
3. Check if processed:
   ```bash
   aaroneous query ingestions --limit 5
   ```
4. Check which specialist processed it
5. Verify specialist got XP awarded

**Success Criteria:**
- File processed automatically
- Specialist identified
- XP was awarded
- File moved from inbox to processed/

---

### Exercise 4: Event History (5 min)

**Objective:** Query and analyze system events

**Steps:**
1. View recent events: `aaroneous query events --limit 20`
2. Find the file ingestion event from Exercise 3
3. Find specialist XP award events
4. Filter by specialist: `aaroneous query events --specialist "Merlin" --limit 10`
5. Check event timestamps

**Success Criteria:**
- Can view events
- Can identify ingestion event
- Can filter by specialist
- Understand event format

---

### Exercise 5: System Health Monitoring (5 min)

**Objective:** Understand system metrics and health

**Steps:**
1. Check health: `aaroneous status health --detailed`
2. Record health percentage
3. View metrics: `aaroneous status metrics --resources`
4. Record memory usage
5. Run continuous monitor: `aaroneous status health --watch 5`
6. Press Ctrl+C after 30 seconds
7. Note if health/memory changed

**Success Criteria:**
- Can retrieve health information
- Can monitor in real-time
- Understand baseline metrics
- Can spot changes

---

## 📋 Training Completion Checklist

**After completing this training, you should be able to:**

- [ ] Start and stop the hive (`aaroneous start --dashboard tui`, Q to quit)
- [ ] Navigate TUI dashboard (Tab to switch pages, ↑/↓ to scroll)
- [ ] List specialists and check their status
- [ ] Award XP to specialists
- [ ] View system statistics and health
- [ ] Drop files in inbox for processing
- [ ] Query events and ingestion history
- [ ] Monitor system metrics and health
- [ ] Troubleshoot basic issues using CLI
- [ ] Find help using `--help` flags

**Next Steps:**
- Run all 5 exercises to build confidence
- Refer to QUICK_START_GUIDE.md for quick reference
- Reference OPERATIONAL_RUNBOOK.md for detailed procedures
- Contact R&D team lead with questions

---

## 🎯 Quick Reference

### Essential Commands

```bash
# System
aaroneous start --dashboard tui        # Start with dashboard
aaroneous status health                # Quick health check
aaroneous status metrics               # View metrics

# Specialists
aaroneous specialist list              # List all specialists
aaroneous specialist status --name "X" # Check specific specialist
aaroneous specialist award --specialist "X" --amount 100 --reason "Task"

# Query
aaroneous query stats                  # System statistics
aaroneous query events --limit 20      # Recent events
aaroneous query ingestions --limit 10  # Ingestion history

# Configuration
aaroneous config show                  # View configuration
aaroneous config validate              # Check validity
```

### Dashboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Next page |
| `Shift+Tab` | Previous page |
| `↑/↓` | Scroll |
| `Q` | Quit |
| `?` | Help |

---

**Version:** 1.0  
**Created:** 2024-01-29  
**For:** Aaroneous R&D Team

**Ready to start?**
```bash
aaroneous start --dashboard tui
```
