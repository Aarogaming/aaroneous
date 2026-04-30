# Aaroneous Operational Runbook
## System Administration and Operations Guide

---

## 📋 Table of Contents

1. [Daily Operations](#daily-operations)
2. [System Monitoring](#system-monitoring)
3. [Incident Response](#incident-response)
4. [Data Management](#data-management)
5. [Performance Tuning](#performance-tuning)
6. [Backup & Recovery](#backup--recovery)
7. [Common Procedures](#common-procedures)

---

## Daily Operations

### Morning Startup (Every Day)

```bash
# 1. Verify system status
aaroneous status health

# Expected output:
# System Health: 85%
# Uptime: 0h 0m
# Specialists: 6/6 active
# Events: 0
# Errors: 0

# 2. Start the hive with TUI dashboard
aaroneous start --dashboard tui

# 3. Monitor for 5 minutes
# - Check all 5 dashboard pages load
# - Verify no errors in Event Log
# - Confirm all 6 specialists present
```

### Evening Shutdown

```bash
# Press 'Q' in dashboard or:
aaroneous stop --graceful

# Verify clean shutdown:
# - No database locks
# - All pending events flushed
# - Hive state saved
```

### Monitoring During Day

**Every 2 hours:**
```bash
aaroneous status metrics --watch 5
# Checks: XP generation, skill progression, ingestion rate
```

**Every 4 hours:**
```bash
aaroneous query stats --detailed
# Full system statistics and specialist progression
```

---

## System Monitoring

### Health Check Dashboard

In TUI, navigate to **Settings** page for real-time metrics:

| Metric | Green | Yellow | Red |
|--------|-------|--------|-----|
| System Health | >80% | 60-80% | <60% |
| Uptime | >24h | 6-24h | <6h |
| Memory Usage | <50% | 50-75% | >75% |
| Event Queue | <100 | 100-500 | >500 |
| Error Rate | 0% | <5% | >5% |

### CLI Health Monitoring

```bash
# Basic health check
aaroneous status health

# Detailed health with thresholds
aaroneous status health --detailed

# Continuous monitoring (updates every 5s)
aaroneous status health --watch 5

# JSON output for parsing
aaroneous status health --json
```

### Resource Monitoring

```bash
# Check CPU, memory, disk usage
aaroneous status metrics --resources

# Check database size
aaroneous status metrics --database-size

# Check event queue depth
aaroneous status metrics --queue-depth
```

### Log Monitoring

```bash
# View recent logs (last 100 lines)
aaroneous query logs --limit 100

# View errors only
aaroneous query logs --level error --limit 50

# View logs from last 1 hour
aaroneous query logs --since 1h

# Stream live logs
aaroneous query logs --follow --level info
```

---

## Incident Response

### Database Locked Error

**Symptoms:**
- "Database file locked" error in logs
- Dashboard freezes
- CLI commands timeout

**Recovery:**
```bash
# 1. Identify process holding lock
tasklist | findstr aaroneous

# 2. Kill all aaroneous processes (Windows)
taskkill /F /IM aaroneous.exe

# 3. Wait 5 seconds
Start-Sleep -Seconds 5

# 4. Restart
aaroneous start --dashboard tui

# 5. If persists, check file permissions
icacls D:\Aaroneous\hive.db
```

### High Memory Usage (>500MB)

**Symptoms:**
- System Health drops below 60%
- Performance degradation
- Events processing slows

**Recovery:**
```bash
# 1. Check memory metrics
aaroneous status metrics --resources

# 2. Identify event queue backlog
aaroneous status metrics --queue-depth

# 3. If queue >1000, pause ingestion temporarily
aaroneous config set inbox.watch false

# 4. Wait for queue to drain (monitor with --watch)
aaroneous status metrics --watch 2

# 5. Resume ingestion
aaroneous config set inbox.watch true
```

### Specialist Stuck in Processing

**Symptoms:**
- Specialist hasn't updated XP in >1 hour
- Event Log shows no recent activity
- Status shows "processing" for long time

**Recovery:**
```bash
# 1. Check specialist status
aaroneous specialist status --name "SpecialistName"

# 2. View recent events for specialist
aaroneous query events --specialist "SpecialistName" --limit 20

# 3. If truly stuck, reset specialist state
aaroneous specialist reset --name "SpecialistName" --force

# 4. Verify recovery
aaroneous specialist status --name "SpecialistName"
```

### File Not Being Processed

**Symptoms:**
- File in inbox doesn't move to processed/
- No events created for dropped file
- File watcher seems inactive

**Recovery:**
```bash
# 1. Verify file watcher is active
aaroneous status health

# 2. Check file format is supported
aaroneous config show --all | findstr supported_formats

# 3. Verify inbox permissions
icacls D:\Aaroneous\inbox

# 4. Check for processing errors
aaroneous query logs --level error --since 1h

# 5. Manually trigger ingestion
aaroneous ingestion process --file "path/to/file"

# 6. If still fails, move file and try smaller sample
# This helps isolate the issue
```

### Corrupted Database

**Symptoms:**
- Database read errors in logs
- SQL exceptions in CLI output
- Dashboard shows no data

**Recovery:**
```bash
# 1. Backup corrupted database
Copy-Item D:\Aaroneous\hive.db D:\Aaroneous\hive.db.corrupted

# 2. Restore from backup
Copy-Item D:\Aaroneous\backups\hive.db.backup D:\Aaroneous\hive.db

# 3. Verify restoration
aaroneous status health

# 4. If no backup available, reinitialize
aaroneous init --reset --force
# WARNING: This creates empty hive, losing all data
```

---

## Data Management

### Database Maintenance

**Weekly maintenance:**
```bash
# Defragment database (improves query performance)
aaroneous maintenance vacuum

# Verify database integrity
aaroneous maintenance check-integrity

# Analyze query statistics
aaroneous maintenance analyze
```

### Event Log Cleanup

```bash
# Archive old events (older than 30 days)
aaroneous maintenance archive-events --older-than 30d

# Purge very old events (older than 90 days)
aaroneous maintenance purge-events --older-than 90d --force

# Export events before cleanup
aaroneous query events --export csv > events_export.csv
```

### Specialist Data Export

```bash
# Export single specialist
aaroneous query specialist --name "Merlin" --export json > merlin.json

# Export all specialists
aaroneous query specialists --export json > all_specialists.json

# Export with full history
aaroneous query specialists --include-history --export json > specialists_full.json
```

### Backup Operations

**Daily backup:**
```bash
# Automatic backup (recommended)
aaroneous backup create --auto-schedule daily

# Manual backup
aaroneous backup create --output D:\Aaroneous\backups\hive_$(date +%Y%m%d_%H%M%S).db

# List backups
aaroneous backup list

# Restore from backup
aaroneous backup restore --from D:\Aaroneous\backups\hive_20240101_120000.db
```

---

## Performance Tuning

### Query Performance

```bash
# Enable query profiling
aaroneous config set database.profile-queries true

# Run query and check timing
aaroneous query stats --profile

# Disable profiling when done
aaroneous config set database.profile-queries false
```

### Connection Pool Tuning

```bash
# Check pool settings
aaroneous config show database.pool

# Increase pool size if many concurrent queries
aaroneous config set database.pool-size 10

# Adjust connection timeout
aaroneous config set database.connection-timeout 30s
```

### Event Processing Optimization

```bash
# Check event processing rate
aaroneous status metrics --event-rate

# Increase processing threads if needed
aaroneous config set processing.threads 4

# Adjust batch size for ingestion
aaroneous config set ingestion.batch-size 100
```

### Memory Management

```bash
# Check memory usage
aaroneous status metrics --memory

# Enable memory profiling
aaroneous config set profiling.memory true

# Monitor memory over time
aaroneous status metrics --memory --watch 5 --duration 60
```

---

## Backup & Recovery

### Backup Strategy

**Daily Backups:**
```bash
# Create backup at specific time (e.g., 2 AM)
# Add to Windows Task Scheduler or cron:
aaroneous backup create --output "D:\Aaroneous\backups\hive_$(date +\%Y\%m\%d).db"
```

**Weekly Full Export:**
```bash
# Export all data as JSON for archival
aaroneous export full --output "D:\Aaroneous\exports\full_$(date +\%Y\%m\%d).json"
```

**Monthly Verification:**
```bash
# Test restore from oldest backup
aaroneous backup restore --from "D:\Aaroneous\backups\hive_FIRST_BACKUP.db" --dry-run
```

### Disaster Recovery

**Scenario: Complete Data Loss**

```bash
# 1. Identify latest good backup
ls -lt D:\Aaroneous\backups\

# 2. Create new clean database
aaroneous init --reset --force

# 3. Restore from backup
aaroneous backup restore --from "D:\Aaroneous\backups\hive_LATEST.db"

# 4. Verify restoration
aaroneous status health
aaroneous query stats

# 5. Resume normal operations
aaroneous start --dashboard tui
```

**Scenario: Partial Data Corruption**

```bash
# 1. Export healthy data
aaroneous export specialists > specialists_good.json

# 2. Create fresh database
aaroneous init --reset --force

# 3. Reimport good data
aaroneous import specialists < specialists_good.json

# 4. Verify and resume
aaroneous status health
aaroneous start --dashboard tui
```

---

## Common Procedures

### Adding a New Specialist

```bash
# Create specialist
aaroneous specialist create \
  --name "NewSpecialist" \
  --archetype "Scholar" \
  --initial-xp 500

# Verify creation
aaroneous specialist status --name "NewSpecialist"

# Check in dashboard
# - Navigate to "Specialists" page
# - Should appear in list
```

### Bulk XP Award

```bash
# Award XP to multiple specialists
aaroneous specialist award \
  --specialist "Ariel,Merlin,Odin" \
  --amount 250 \
  --reason "Monthly achievement"

# Verify awards
aaroneous query stats --detailed
```

### Processing Data File Manually

```bash
# 1. Place file in inbox
cp mydata.csv D:\Aaroneous\inbox\

# 2. Wait for automatic processing (10 seconds typical)
Start-Sleep -Seconds 10

# 3. Verify processing
aaroneous query ingestions --limit 5

# 4. Check specialist XP increase
aaroneous query specialist --name "YourSpecialist"
```

### Analyzing Performance

```bash
# 1. Collect metrics over time
aaroneous status metrics --watch 2 --duration 300 > metrics.txt

# 2. Check average values
cat metrics.txt | grep "Event Rate"

# 3. Identify bottlenecks
aaroneous status health --detailed | findstr "WARNING\|ERROR"
```

### Configuring File Watch Patterns

```bash
# View current patterns
aaroneous config show inbox.watch-patterns

# Add pattern for GGUF files
aaroneous config add-pattern --extension gguf

# Remove pattern for specific extension
aaroneous config remove-pattern --extension tmp

# Verify patterns
aaroneous config show inbox.watch-patterns
```

### Updating Specialist Configuration

```bash
# View specialist config
aaroneous specialist config --name "Merlin"

# Update specialist archetype
aaroneous specialist update --name "Merlin" --archetype "Expert"

# Reset specialist skills
aaroneous specialist reset-skills --name "Merlin" --force

# Verify update
aaroneous specialist config --name "Merlin"
```

---

## Troubleshooting Reference

| Issue | Command to Run | Expected Outcome |
|-------|----------------|------------------|
| System won't start | `aaroneous status health` | Shows health percentage |
| Can't see specialists | `aaroneous specialist list` | Lists 6 specialists |
| No events appearing | `aaroneous query logs --level error` | Shows any error logs |
| Dashboard frozen | Check logs, restart via `aaroneous stop --graceful` | Clean shutdown |
| High memory | `aaroneous status metrics --resources` | Shows current memory usage |
| Slow performance | `aaroneous maintenance analyze` | Optimizes queries |
| Data questions | `aaroneous query stats --detailed` | Full system statistics |

---

## Support and Escalation

### When to Escalate

- Database corruption that backup restore doesn't fix
- Persistent memory leaks (>80% usage continuously)
- Data loss incidents
- Security concerns
- Multi-day outages

### Information to Gather

```bash
# Collect support bundle
aaroneous support bundle

# This creates: support_bundle_TIMESTAMP.zip containing:
# - System logs (last 24h)
# - Configuration (sanitized)
# - Database schema and statistics
# - Recent events (last 100)
# - System metrics snapshot
```

### Contact Information

**For operational support:**
- R&D Team Lead
- Platform Engineering

**For escalations:**
- Architecture Team

---

## Checklists

### Weekly Operations Checklist

- [ ] Run `aaroneous status health` - confirm >80%
- [ ] Check Event Log for errors - should be <1%
- [ ] Run `aaroneous maintenance vacuum`
- [ ] Verify backup completed successfully
- [ ] Export events for archival
- [ ] Review specialist progression
- [ ] Check ingestion statistics
- [ ] Verify no database corruption (`aaroneous maintenance check-integrity`)

### Monthly Operations Checklist

- [ ] Full system backup and test restore
- [ ] Archive and purge old events
- [ ] Analyze query performance (`aaroneous maintenance analyze`)
- [ ] Review and optimize configuration settings
- [ ] Export full dataset for offsite storage
- [ ] Performance analysis and tuning
- [ ] Security audit of configurations
- [ ] Team training update (if needed)

---

**Version:** 1.0  
**Last Updated:** 2024-01-29  
**Maintained By:** R&D Operations Team
