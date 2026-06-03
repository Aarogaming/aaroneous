# AARONEOUS PRODUCTION OPERATIONS GUIDE

**Version**: 1.0 (97% System)  
**Date**: June 1, 2026  
**Status**: Production Ready  
**Audience**: DevOps, Operations, Support Teams

---

## 📋 TABLE OF CONTENTS

1. [System Architecture](#system-architecture)
2. [Deployment Guide](#deployment-guide)
3. [Configuration](#configuration)
4. [Operations](#operations)
5. [Monitoring](#monitoring)
6. [Troubleshooting](#troubleshooting)
7. [Disaster Recovery](#disaster-recovery)
8. [Performance Tuning](#performance-tuning)

---

## 🏗️ SYSTEM ARCHITECTURE

### Single Instance (Current - 97%)
```
┌─────────────────────────────────┐
│   Aaroneous Single Instance     │
├─────────────────────────────────┤
│ Autonomic Loop (Main Heartbeat) │
├─────────────────────────────────┤
│ Thermal Management              │
│ Task Routing                    │
│ Learning System                 │
│ Token Budgeting                 │
├─────────────────────────────────┤
│ Advanced Features               │
│ - Load Balancing                │
│ - Learning Rate Optimization    │
│ - Checkpointing                 │
├─────────────────────────────────┤
│ Storage Layer                   │
│ - Registry Persistence          │
│ - State Checkpoints             │
└─────────────────────────────────┘
```

### Core Components
- **AutonomicLoop**: Main heartbeat (2-6ms per cycle)
- **SystemMetricsCollector**: Thermal & system monitoring
- **TaskRouter**: Intelligent task routing
- **UnifiedLearningLoop**: Model training & updates
- **PredictiveLoadBalancer**: Workload forecasting
- **AdaptiveLearningOptimizer**: Learning rate tuning
- **DistributedCheckpointManager**: State persistence

---

## 🚀 DEPLOYMENT GUIDE

### Pre-Deployment Checklist

```
System Requirements:
  [ ] Linux 5.10+ or macOS 11+
  [ ] 4+ CPU cores recommended
  [ ] 8GB+ RAM recommended
  [ ] 100MB+ disk for logs
  
Build & Test:
  [ ] cargo build --release
  [ ] cargo test -p hypervisor
  [ ] cargo test -p biology
  [ ] All 126+ tests passing
  
Configuration:
  [ ] config.toml created
  [ ] Log directory ready
  [ ] Metrics directory ready
  [ ] Checkpoint directory ready
```

### Single Instance Deployment

#### Step 1: Build Release Binary
```bash
cd D:\Aaroneous
cargo build --release -p hypervisor

# Binary location: target/release/aaroneous-hypervisor
# Size: ~50-100MB
# Build time: 5-15 minutes
```

#### Step 2: Create Configuration
```toml
# config.toml
[server]
port = 8080
tick_rate_ms = 100

[thermal]
cpu_warning_temp = 75
cpu_critical_temp = 90
gpu_warning_temp = 80
gpu_critical_temp = 95

[biology]
expression_rate_normal = 1.0
expression_rate_thermal = 0.7
expression_rate_emergency = 0.5

[learning]
learning_rate_min = 0.00001
learning_rate_max = 0.1
checkpoint_interval = 100

[storage]
registry_dir = "/var/aaroneous/registry"
checkpoint_dir = "/var/aaroneous/checkpoints"
log_dir = "/var/log/aaroneous"
```

#### Step 3: Deploy Binary
```bash
# Create deployment directory
mkdir -p /opt/aaroneous/{bin,config,logs}

# Copy binary
cp target/release/aaroneous-hypervisor /opt/aaroneous/bin/

# Copy config
cp config.toml /opt/aaroneous/config/

# Set permissions
chmod +x /opt/aaroneous/bin/aaroneous-hypervisor
chmod 644 /opt/aaroneous/config/config.toml
```

#### Step 4: Start Service
```bash
# Manual start (for testing)
/opt/aaroneous/bin/aaroneous-hypervisor \
  --config /opt/aaroneous/config/config.toml \
  --log-level debug

# Systemd service (for production)
# See systemd-service.conf section below
```

---

## ⚙️ CONFIGURATION

### Thermal Management

```toml
[thermal]
# Temperature thresholds (Celsius)
cpu_warning_temp = 75      # Start monitoring closely
cpu_throttle_temp = 85     # Reduce expression_rate to 0.7x
cpu_critical_temp = 90     # Emergency throttle to 0.5x

gpu_warning_temp = 80
gpu_throttle_temp = 85
gpu_critical_temp = 95

# Throttle factors (0.0-1.0)
throttle_factor_normal = 1.0
throttle_factor_warm = 0.9
throttle_factor_hot = 0.7
throttle_factor_critical = 0.5
```

### Learning Configuration

```toml
[learning]
# Learning rate bounds
learning_rate_min = 0.00001
learning_rate_max = 0.1
learning_rate_initial = 0.001

# Convergence tracking
convergence_window = 20      # Steps to analyze trend
convergence_threshold = 0.01 # Loss improvement threshold

# Checkpointing
checkpoint_interval = 100    # Every N learning steps
checkpoint_dir = "/var/aaroneous/checkpoints"
max_checkpoints = 50         # Keep last 50
```

### Task Routing

```toml
[routing]
# Route-specific throttle factors
enzyme_priority = 1.3
network_priority = 1.1
learning_priority = 0.9
cpu_priority = 1.2
memory_priority = 0.8

# Load balancing
prediction_window_secs = 10
queue_depth_threshold = 5
specialist_timeout_secs = 30
```

---

## 📊 OPERATIONS

### Starting the System

#### Manual Start (Development)
```bash
RUST_LOG=debug /opt/aaroneous/bin/aaroneous-hypervisor \
  --config config.toml \
  --mode single-instance
```

#### Systemd Service (Production)
```ini
# /etc/systemd/system/aaroneous.service
[Unit]
Description=Aaroneous Autonomous Agent System
After=network.target

[Service]
Type=simple
User=aaroneous
WorkingDirectory=/opt/aaroneous
ExecStart=/opt/aaroneous/bin/aaroneous-hypervisor \
  --config /opt/aaroneous/config/config.toml
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=multi-user.target
```

#### Enable and Start
```bash
systemctl daemon-reload
systemctl enable aaroneous
systemctl start aaroneous
systemctl status aaroneous
```

### Stopping the System

#### Graceful Shutdown
```bash
systemctl stop aaroneous

# Timeout for graceful shutdown (default 30s)
# System will:
# 1. Pause autonomic loop
# 2. Finalize pending checkpoints
# 3. Persist state
# 4. Close connections
# 5. Exit cleanly
```

#### Force Stop (Emergency)
```bash
systemctl kill aaroneous

# Warning: May lose in-flight state
# Use only if graceful shutdown fails
```

### Log Management

```bash
# View logs (last 100 lines)
journalctl -u aaroneous -n 100 -f

# View logs with level filter
journalctl -u aaroneous -p err  # Errors only
journalctl -u aaroneous -p info # Info and above

# Export logs
journalctl -u aaroneous > aaroneous-logs.txt

# Rotate logs (if using file logging)
logrotate /etc/logrotate.d/aaroneous
```

---

## 📈 MONITORING

### Key Metrics to Monitor

```
System Metrics:
  - autonomic_tick_time_ms     (target: < 10ms)
  - thermal_throttle_factor    (target: > 0.8)
  - specialist_queue_depth     (target: < 5)
  - learning_convergence_rate  (target: improving)
  - checkpoint_success_rate    (target: > 99%)
  
Performance Metrics:
  - tasks_executed_per_sec     (target: > 100)
  - average_task_latency_ms    (target: < 50ms)
  - learning_iterations        (target: > 1000/session)
  - model_improvement_percent  (target: > 0.1%/session)
```

### Health Checks

```bash
# System health endpoint (add in Phase 8)
curl http://localhost:8080/health

# Response:
{
  "status": "healthy",
  "uptime_seconds": 3600,
  "autonomic_loop_running": true,
  "thermal_status": "normal",
  "learning_converged": true,
  "checkpoints_stable": true
}
```

### Metrics Collection

```
Metrics Exported (Prometheus format):
  - aaroneous_autonomic_tick_seconds
  - aaroneous_thermal_throttle_factor
  - aaroneous_specialist_queue_depth
  - aaroneous_learning_loss
  - aaroneous_checkpoint_latency_ms
  
Scrape interval: 60 seconds
Retention: 30 days
```

---

## 🔧 TROUBLESHOOTING

### Issue: High Thermal Throttling

**Symptoms**: Throttle factor < 0.7, reduced throughput

**Diagnosis**:
```bash
# Check temperatures
journalctl -u aaroneous | grep -i thermal

# Monitor in real-time
watch -n 1 "systemctl status aaroneous | grep -i temp"
```

**Solutions**:
1. Improve cooling (increase ventilation)
2. Reduce load (fewer concurrent tasks)
3. Increase tick_rate_ms (slower loop)
4. Enable load shedding (defer non-critical tasks)

---

### Issue: Learning Not Converging

**Symptoms**: Loss plateau, no improvement for 1000+ steps

**Diagnosis**:
```bash
# Check learning logs
journalctl -u aaroneous | grep -i learning

# Check checkpoint health
ls -lh /var/aaroneous/checkpoints/
```

**Solutions**:
1. Check learning rate (should auto-tune)
2. Verify task diversity (different task types)
3. Check for data quality issues
4. Reset learning with fresh checkpoint

---

### Issue: Memory Usage Growing

**Symptoms**: RAM usage > 80%, performance degradation

**Diagnosis**:
```bash
# Monitor memory
watch -n 5 "ps aux | grep aaroneous"

# Check for memory leaks
systemctl restart aaroneous  # Restart cycle
```

**Solutions**:
1. Reduce checkpoint history (max_checkpoints)
2. Clear old logs/metrics
3. Reduce task queue size
4. Enable memory pooling (future optimization)

---

## 💾 DISASTER RECOVERY

### Backup Strategy

```bash
# Daily backup (add to cron)
0 2 * * * /opt/aaroneous/backup.sh

# Backup script:
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
tar -czf /backup/aaroneous_$DATE.tar.gz \
  /var/aaroneous/checkpoints/ \
  /var/aaroneous/registry/ \
  /opt/aaroneous/config/
```

### Recovery Procedure

```bash
# 1. Stop service
systemctl stop aaroneous

# 2. Restore from backup
tar -xzf /backup/aaroneous_20260601_020000.tar.gz \
  -C /

# 3. Verify integrity
ls -lh /var/aaroneous/checkpoints/
md5sum /var/aaroneous/checkpoints/*

# 4. Start service
systemctl start aaroneous

# 5. Monitor recovery
journalctl -u aaroneous -f
```

---

## ⚡ PERFORMANCE TUNING

### Tuning Parameters

```toml
# Autonomic Loop Frequency
[autonomic]
tick_rate_ms = 100          # 100ms = 10 cycles/sec (good balance)
# Decrease for lower latency, increase for lower CPU

# Specialist Allocation
[specialists]
enzyme_runners = 2          # Concurrent WASM executors
network_executors = 2       # Concurrent network tasks
learning_workers = 1        # Learning worker threads

# Load Balancing
[load_balancer]
prediction_window_secs = 10 # Look ahead 10 seconds
queue_threshold = 5         # Warn at 5 tasks queued
```

### Performance Targets

```
Single Instance (97%):
  - Autonomic tick:     2-6ms
  - Route decision:     <5μs
  - Learning update:    200-500μs
  - Checkpoint write:   500-1500μs
  - Throughput:         50-100 tasks/sec
  - Latency p99:        < 50ms

With SIMD (Phase 7):
  - Learning update:    100-250μs (2x faster)
  - Throughput:         100-200 tasks/sec
  - Overall improvement: 20%+
```

---

## 📞 SUPPORT & ESCALATION

### Support Contacts
- **Level 1**: Operations team (on-call)
- **Level 2**: DevOps engineer (escalation)
- **Level 3**: Core developers (critical issues)

### Escalation Criteria
- System down > 5 minutes
- Data loss detected
- Thermal emergency
- Learning divergence

### Emergency Procedures
```bash
# Immediate action for critical issues:

# 1. Check system status
systemctl status aaroneous

# 2. Collect diagnostics
mkdir /tmp/aaroneous-diag
journalctl -u aaroneous > /tmp/aaroneous-diag/logs.txt
ps aux > /tmp/aaroneous-diag/processes.txt
free -h > /tmp/aaroneous-diag/memory.txt

# 3. Attempt recovery
systemctl restart aaroneous

# 4. Escalate if needed
tar -czf /tmp/aaroneous-diag.tar.gz /tmp/aaroneous-diag/
```

---

## 📚 ADDITIONAL RESOURCES

- **Architecture Guide**: PHASE_6_HA_IMPLEMENTATION.md
- **Performance Tuning**: docs/performance-tuning.md
- **Learning System**: UNIFIED_LEARNING_DOPAMINE_TRAINING.md
- **API Reference**: docs/api-reference.md
- **Runbooks**: docs/runbooks/

---

**Operations Guide Status**: Production Ready ✅  
**Last Updated**: June 1, 2026  
**Next Update**: After Phase 6 HA implementation

