# FEDERATION DEPLOYMENT CHECKLIST

**Product:** Aaroneous Hive (Agent-Zero)  
**Version:** 0.1.0 (Epoch VI - Production Finalization)  
**Date:** 2026-04-28  
**Status:** Ready for Validation

---

## Pre-Deployment Validation (Dev/Staging)

### Code Integrity

- [ ] **Rust compilation clean**
  ```powershell
  cd D:\Aaroneous
  cargo build --release
  ```
  Expected: Zero warnings, no errors

- [ ] **All modules present**
  - [ ] `src/agents.rs` – Agent taxonomy
  - [ ] `src/biology.rs` – Metabolic governance
  - [ ] `src/shared_memory.rs` – Zero-copy IPC
  - [ ] `src/enzymes.rs` – Enzyme abstraction
  - [ ] `src/bin/main.rs` – Kernel entry point

- [ ] **HOX configuration complete**
  - [ ] `registry/hox_map.json` – AlphaNode baseline
  - [ ] `registry/hox_specialist_*.json` (6 files) – Ariel, Merlin, Odin, Dionysus, Hephaestus, Argus
  - [ ] `registry/hox_relic_*.json` (6 files) – Glass, Grimoire, Draupnir, Omni, Forge, Sentinel

- [ ] **Chromosome library present**
  - [ ] `chromosomes/nat_bridge.dll`
  - [ ] `chromosomes/tensor_forge.dll`
  - [ ] `chromosomes/thought_kernel.dll`
  - [ ] `chromosomes/sensor_node.dll`
  - [ ] `chromosomes/wasm_enzyme.wasm`

### Functional Testing

- [ ] **Unit Tests Pass**
  ```powershell
  cargo test --lib
  ```
  Expected: All biology, agents, and enzyme tests pass

- [ ] **NATS Broker Connectivity**
  ```powershell
  # Verify NATS server running on localhost:4222
  nats-server --version
  ```
  Expected: NATS accessible and responsive

- [ ] **Enzyme Loading**
  ```powershell
  .\bin\a-run.exe --console
  # Monitor first 30 seconds
  # Should see "Enzyme loaded: <name>" messages
  ```
  Expected: All 4 DLLs + 1 WASM load without errors

- [ ] **Token-Bucket Metabolism**
  ```
  Monitor heartbeat messages:
  - expression_rate should be 1.0
  - tokens should increase over time
  - throttle_state should be "Normal"
  ```
  Expected: Smooth token regeneration every 5 seconds

- [ ] **Specialist Spawning (Manual)**
  ```json
  Publish to federation.control.spawn_specialist:
  {"name": "ariel", "activate": true}
  
  Verify:
  - federation.specialist.ariel.spawned published
  - Ariel begins reporting to federation.specialist.ariel.report
  - Glass (relic) loads in Ariel's context
  ```
  Expected: Ariel and Glass both active within 100ms

- [ ] **Expression Rate Adjustment**
  ```json
  Publish to federation.control.set_expression_rate:
  {"rate": 0.5}
  
  Verify:
  - Specialist execution intervals double
  - Throttle state transitions to "Metabolic"
  - Token regeneration slows proportionally
  ```
  Expected: All specialists respond within 1 second

- [ ] **Specialist Halt**
  ```json
  Publish to federation.control.halt_specialist:
  {"name": "ariel"}
  
  Verify:
  - federation.specialist.ariel.halted published
  - Ariel task terminates gracefully
  - Glass (supervised relic) also halts
  ```
  Expected: Clean shutdown within 500ms

### Integration Testing

- [ ] **Multi-Specialist Execution**
  - Spawn all 6 specialists simultaneously
  - Monitor for 60 seconds
  - Verify no cross-specialist interference
  - Check execution counts increase steadily

- [ ] **Heavy Load Test**
  - Set expression_rate = 1.0
  - Spawn all 6 specialists
  - Spawn 3 test user agents
  - Run for 5 minutes
  - Expected: No memory leaks, token metabolism stable

- [ ] **Enzyme Error Handling**
  - Corrupt one enzyme DLL checksum
  - Attempt to spawn specialist that requires it
  - Expected: Graceful error, specialist doesn't crash

- [ ] **NATS Bus Resilience**
  - Kill NATS broker
  - A-Run should emit error and attempt reconnect
  - Restart NATS broker
  - Expected: A-Run recovers and resumes heartbeat

### Security Validation

- [ ] **Token Authentication (Named Pipes)**
  - [ ] IPC authentication layer in place
  - [ ] Unauthorized access rejected
  - [ ] Session tokens properly scoped

- [ ] **Input Validation**
  - [ ] NATS message payloads validated (JSON schema)
  - [ ] HOX files hash-verified before load
  - [ ] Enzyme checksums match registry

- [ ] **Enzyme Isolation**
  - [ ] Each enzyme runs in isolated memory context
  - [ ] Buffer overflow attempt in enzyme doesn't crash kernel
  - [ ] Specialist metadata never leaks between agents

- [ ] **User Permission Enforcement**
  - [ ] Observer users cannot spawn specialists
  - [ ] Operator users cannot adjust expression_rate
  - [ ] Only Administrators can modify HOX files

### Performance Benchmarks

- [ ] **Startup Time**
  - Expected: < 2 seconds from process start to first heartbeat
  - Actual: _____ seconds

- [ ] **Specialist Spawn Latency**
  - Expected: < 100ms from control message to first report
  - Actual: _____ ms

- [ ] **Token Regeneration Accuracy**
  - Spawn specialist with 20s interval
  - Measure actual interval (should be 20s ± 5ms)
  - Actual: _____ ms variance

- [ ] **Memory Footprint**
  - A-Run baseline (no specialists): < 50 MB
  - + Each specialist task: < 25 MB
  - + All 6 specialists: < 200 MB
  - Actual baseline: _____ MB
  - Actual with 6 specialists: _____ MB

- [ ] **CPU Utilization**
  - A-Run idle (expression_rate=0): < 1%
  - A-Run normal (expression_rate=1.0, 6 specialists): < 25%
  - Actual idle: _____ %
  - Actual normal: _____ %

---

## Production Deployment

### Pre-Launch Checklist

- [ ] **Environment Variables Set**
  ```powershell
  $env:AAS_WORKSPACE_ROOT = "D:\"
  $env:NATS_SERVER = "localhost:4222"
  ```

- [ ] **Logs Directory Exists**
  ```powershell
  Test-Path "D:\Aaroneous\logs"
  # Expected: True
  ```

- [ ] **Windows Service Account**
  - [ ] Service runs under NT Authority\System or dedicated account
  - [ ] Account has RW access to D:\Aaroneous\ directory
  - [ ] Account has permission to bind to NATS (if not localhost)

- [ ] **NATS JetStream Configured**
  ```
  - Persistent store for federation.* streams
  - Retention policy: 7 days or 10GB
  - Replication factor: 3 (if clustered)
  ```

- [ ] **Firewall Rules**
  - [ ] NATS broker accessible (default 4222)
  - [ ] Named Pipe endpoints accessible within domain

- [ ] **Backup & Recovery**
  - [ ] `chromosomes/` directory backed up (immutable, but safety)
  - [ ] `registry/hox_*.json` backed up
  - [ ] `logs/` rotation configured (7-day retention)

### Deployment Steps

1. **Prepare Release Binary**
   ```powershell
   cd D:\Aaroneous
   cargo build --release
   Copy-Item "target\release\a_run.exe" "bin\a-run.exe" -Force
   ```

2. **Install Windows Service**
   ```powershell
   .\bin\a-run.exe --install
   # Expected: Service "AaroneousARun" created
   ```

3. **Start Service**
   ```powershell
   net start AaroneousARun
   # Expected: Service started successfully
   ```

4. **Verify Startup**
   ```powershell
   # Wait 5 seconds for initialization
   Start-Sleep -Seconds 5
   
   # Check service status
   Get-Service AaroneousARun
   # Expected: Status = Running
   
   # Check logs
   Get-Content "D:\Aaroneous\logs\arun_core.log" -Tail 20
   # Expected: "federation heartbeat emitted" every 5s
   ```

5. **Test NATS Connectivity**
   ```powershell
   # Publish test message
   nats pub federation.test.ping '{"msg":"deployment_test"}' --server=localhost:4222
   
   # Watch logs for acknowledgment
   Get-Content "D:\Aaroneous\logs\federation_bus.log" -Tail 5
   # Expected: Message received and processed
   ```

6. **Smoke Test: Spawn Specialist**
   ```json
   Subject: federation.control.spawn_specialist
   Payload: {"name": "ariel", "activate": true}
   
   Verify:
   - Check logs for "Specialist Ariel spawned"
   - Monitor federation.specialist.ariel.report for output
   - Expected: At least 3 reports within 60 seconds
   ```

### Post-Launch Monitoring (24 Hours)

- [ ] **Service Stability**
  - [ ] A-Run service has not restarted
  - [ ] No out-of-memory events
  - [ ] No critical enzyme failures in logs

- [ ] **Heartbeat Continuity**
  - [ ] `federation.heartbeat` published every 5s (±100ms)
  - [ ] expression_rate stable at 1.0
  - [ ] throttle_state remains "Normal"

- [ ] **Specialist Health**
  - [ ] All active specialists reporting regularly
  - [ ] No hung or deadlocked tasks
  - [ ] Relic supervision working correctly

- [ ] **User Sessions**
  - [ ] Users connecting and disconnecting cleanly
  - [ ] Session tokens not leaking
  - [ ] Permission enforcement working

- [ ] **Enzyme Execution**
  - [ ] Zero enzyme access violations
  - [ ] No buffer overflows or crashes
  - [ ] Checksums verified on each load

### Rollback Plan

If critical issues discovered post-deployment:

1. **Graceful Shutdown**
   ```powershell
   net stop AaroneousARun
   ```

2. **Restore Previous Binary**
   ```powershell
   Copy-Item "bin\a-run.exe.backup" "bin\a-run.exe" -Force
   ```

3. **Restart Service**
   ```powershell
   net start AaroneousARun
   ```

4. **Verify Rollback**
   ```powershell
   Get-Content "D:\Aaroneous\logs\arun_core.log" -Tail 20
   # Should show previous version in initialization log
   ```

---

## Sign-Off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| **Development Lead** | ________________ | ________________ | ________________ |
| **QA Lead** | ________________ | ________________ | ________________ |
| **Operations Lead** | ________________ | ________________ | ________________ |
| **Security Lead** | ________________ | ________________ | ________________ |

---

## Post-Deployment Notes

Document any issues discovered during deployment:

```
Issue #1: 
Description: 
Resolution: 
Date Resolved: 

Issue #2:
Description:
Resolution:
Date Resolved:
```

---

**Deployment Completed:** ________________  
**Deployed By:** ________________  
**Environment:** (Dev/Staging/Production) ________________  
**Approval:** ________________

---

**Next Phase:** Epoch VII - MaelstromUI Integration & User-Facing Gamification

