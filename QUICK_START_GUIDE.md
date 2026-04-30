# Aaroneous Quick Start Guide
## Get Up and Running in 5 Minutes

---

## 📋 Prerequisites

- Windows 10/11 or Linux/Mac with Rust toolchain
- 500MB free disk space
- Terminal/Command Prompt

---

## 🚀 Getting Started

### 1. Build the Project

```bash
cd D:\Aaroneous
cargo build --release
```

**Time:** ~30 seconds  
**Output:** `target/release/aaroneous.exe`

### 2. Start the Hive

```bash
./target/release/aaroneous start --dashboard tui
```

**What happens:**
- ✅ Initializes SQLite database
- ✅ Starts file watcher for `D:\Aaroneous\inbox`
- ✅ Launches Terminal UI dashboard
- ✅ Shows system health and specialists

### 3. In the Dashboard

**Navigation:**
- `Tab` = Switch pages
- `↑/↓` = Scroll
- `Q` = Quit

**Pages:**
1. **Home** - System health, XP, specialist count
2. **Specialists** - List of 6 active specialists
3. **Skill Tree** - Available skills and levels
4. **Event Log** - Recent activities
5. **Settings** - Configuration view

---

## 🎯 Basic Commands

### Create a Specialist

```bash
aaroneous specialist create \
  --name "YourName" \
  --archetype "Scholar"
```

### List All Specialists

```bash
aaroneous specialist list --detailed
```

### Award XP

```bash
aaroneous specialist award \
  --specialist "YourName" \
  --amount 250 \
  --reason "Data ingestion"
```

### Check System Health

```bash
aaroneous status health
```

### View Statistics

```bash
aaroneous query stats --detailed
```

---

## 📁 Directory Structure

```
D:\Aaroneous\
├── inbox/                 # Drop files here for ingestion
├── processed/             # Completed data files
│   ├── processing/       # Files being processed
│   ├── processed/        # Successfully processed
│   ├── failed/           # Failed files
│   └── analytics/        # Statistics
├── hive.db               # SQLite database (auto-created)
├── logs/                 # Structured logs
└── src/                  # Source code
    ├── cli.rs            # CLI commands
    ├── persistence.rs    # Database layer
    ├── tui_framework.rs  # Dashboard
    └── ...
```

---

## 🔧 Configuration

### Environment Variables

```bash
# Database location
AARONEOUS_DB_PATH=D:\Aaroneous\hive.db

# Inbox location
AARONEOUS_INBOX=D:\Aaroneous\inbox

# Log level (debug, info, warn, error)
AARONEOUS_LOG_LEVEL=info

# Enable JSON logs for parsing
AARONEOUS_JSON_LOGS=true
```

### Via CLI

```bash
aaroneous config show
aaroneous config validate
aaroneous config export --output config.json
```

---

## 📊 Specialist Archetypes

Each specialist has unique characteristics:

| Name | Archetype | Domain | Skills | XP |
|------|-----------|--------|--------|-----|
| Ariel | UI Designer | UserInterface | RAG, API | 2,500 |
| Merlin | Knowledge | Knowledge | DAG, RAG | 2,200 |
| Odin | Leader | Leadership | DAG, MCP | 1,900 |
| Circe | Analyst | Experience | RAG, Fusion | 1,600 |
| Hephaestus | Inventor | Manufacturing | MCP, API | 1,200 |
| Argus | Guardian | Security | API, Unique | 800 |

---

## ⭐ Specialist Ranks

Progress through 5 ranks as specialists gain XP:

```
Rank 1: Newly Digested      (0 XP)
Rank 2: Integrated Specialist  (1,000 XP)
Rank 3: Trusted Member      (3,000 XP)
Rank 4: Domain Expert       (6,000 XP)
Rank 5: Transcendent        (10,000 XP)
```

---

## 💎 Skills

5 skill types available:

- **DAG** - Task decomposition and planning
- **RAG** - Knowledge synthesis and retrieval
- **MCP** - Tool integration and protocol handling
- **API** - Federation and communication
- **Fusion** - Combined skills (created by specialists)

Skills progress from Level 1 to 20.

---

## 📥 Data Ingestion

### Drop Files in Inbox

```bash
# Supported formats
- .gguf (GGUF models)
- .json (Configuration files)
- .csv (Data files)
- .parquet (Data sets)
- .log (Log files)
- And 10+ more...
```

### How It Works

1. Drop file in `D:\Aaroneous\inbox/`
2. File watcher detects it
3. Content analyzed automatically
4. Routed to appropriate specialist
5. XP awarded based on quality
6. File moved to `processed/`

### Example

```bash
# Copy a model file
cp model.gguf D:\Aaroneous\inbox\

# Check status in CLI
aaroneous query ingestions --specialist "Merlin"
```

---

## 🐛 Troubleshooting

### "Database locked" error
```bash
# Ensure no other instances running
# Kill any stuck processes
# Try again
```

### File not being processed
```bash
# Check file format is supported
aaroneous config show --all
# View recent events
aaroneous query events --limit 50
```

### Performance issues
```bash
# Check system health
aaroneous status health --watch 5

# View resource usage
aaroneous status metrics --resources
```

### Logs not appearing
```bash
# Enable JSON logs
aaroneous status health --json-logs
# Check log directory
ls D:\Aaroneous\logs\
```

---

## 🆘 Getting Help

### View Command Help

```bash
aaroneous --help
aaroneous specialist --help
aaroneous query --help
aaroneous status --help
```

### Check Documentation

- **Setup Issues:** See TROUBLESHOOTING.md
- **Architecture:** See FINAL_STATUS_REPORT.md
- **Commands:** Run `aaroneous --help`
- **Data Formats:** See DATA_FORMAT_SPEC.md

---

## ✅ Verification Checklist

After starting, verify:

- [ ] Dashboard displays (5 pages)
- [ ] Can see 6 specialists listed
- [ ] System health shows ~85%
- [ ] CLI commands respond
- [ ] File watcher monitoring inbox
- [ ] No errors in logs

---

## 🎯 Next Steps

1. **Explore Dashboard** - Switch through 5 pages
2. **Create a Specialist** - Use CLI to add your own
3. **Award XP** - Give specialists experience
4. **Check Events** - View activity in Event Log
5. **Drop a File** - Test data ingestion

---

## 📞 Support

For issues or questions:
1. Check logs: `D:\Aaroneous\logs/`
2. View recent events: `aaroneous query events`
3. Check health: `aaroneous status health`
4. Review documentation files
5. Contact R&D team with logs

---

**Ready to explore Aaroneous?**

```bash
aaroneous start --dashboard tui
```

Enjoy! 🚀
