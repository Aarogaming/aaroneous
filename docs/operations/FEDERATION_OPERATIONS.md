# FEDERATION OPERATIONS: Aaroneous Hive Management

## Overview

Aaroneous (Agent-Zero) is the foundational orchestrator of the synthetic intelligence hive. The system operates as a single binary process with parallel internal state generators for specialists and relics, coordinated via NATS message bus and binary enzyme execution.

## System Architecture

### Agent Hierarchy

```
Aaroneous (Agent-Zero) [BaseAgent]
├── Specialists (6 Interactive Personifications) [SpecialistAgent]
│   ├── Ariel (UserInterface Domain) → supervises Glass (Relic)
│   ├── Merlin (Knowledge Domain) → supervises Grimoire (Relic)
│   ├── Odin (Leadership Domain) → supervises Draupnir (Relic)
│   ├── Dionysus (Experience Domain) → supervises Omni (Relic)
│   ├── Hephaestus (Manufacturing Domain) → supervises Forge (Relic)
│   └── Argus (Security Domain) → supervises Sentinel (Relic)
├── Relics (6 Smart Artifacts) [RelicAgent]
│   ├── Glass (Visual Operator - supervised by Ariel)
│   ├── Grimoire (Prophetic Knowledge Index - supervised by Merlin)
│   ├── Draupnir (Resource Allocator - supervised by Odin)
│   ├── Omni (Experience Librarian - supervised by Dionysus)
│   ├── Forge (Manufacturing Executor - supervised by Hephaestus)
│   └── Sentinel (Security Monitor - supervised by Argus)
└── Users (Human Participants) [UserAgent]
    └── Session-based agent instances for user interaction
```

### Execution Model

- **Single Process:** All agents run within A-Run kernel as tokio async tasks
- **Zero-Copy IPC:** Specialist ↔ Enzyme communication via SharedMemorySynapse (memory-mapped buffers)
- **Token-Bucket Metabolism:** Global expression_rate throttles all specialists proportionally
- **HOX Phenotyping:** Each specialist/relic loads its HOX preset for immutable identity
- **Federation Bus:** NATS JetStream for inter-node and control messages

## Specialist Lifecycle

### 1. Startup (Boot-Time)

When A-Run initializes with `--console` or as Windows Service:

```
1. Load Aaroneous BaseAgent (Agent-Zero)
2. Initialize SystemBiology with global expression_rate=1.0
3. Parse hox_map.json (AlphaNode baseline)
4. Load all enforced enzymes into cache/
5. [Optional] Spawn specialist instances if configured in startup profile
6. Connect to NATS broker (localhost:4222)
7. Begin emitting federation.heartbeat every 5 seconds
```

### 2. Spawning a Specialist

**Via NATS Control Message:**

```json
{
  "subject": "federation.control.spawn_specialist",
  "payload": {
    "name": "ariel",
    "activate": true,
    "user_id": "user_123"
  }
}
```

**Kernel Actions:**

1. Call `create_specialist("ariel")` → produces SpecialistAgent
2. Load hox_specialist_ariel.json
3. Register in SystemBiology metabolism with interval_ms=20000
4. Spawn tokio::task for specialist's event loop
5. Load supervised relic (Glass) in same task context
6. Publish `federation.specialist.ariel.spawned` with metadata

### 3. Specialist Active Loop

```
Every 20ms (configurable via interval_ms):
├── Check if metabolic tokens available (can_execute_specialist)
├── If tokens available:
│   ├── Consume token (consume_specialist_token)
│   ├── Load specialist HOX phenotype
│   ├── Invoke enzyme(s) from enzyme_subset via aas_process
│   ├── Collect findings
│   ├── Publish to federation.specialist.<name>.report
│   └── Update execution_count
└── If tokens unavailable:
    └── Enter metabolic depression (await next regen cycle)
```

### 4. Relic Supervision

Relics are **not spawned independently**. They are:

- Loaded by their supervising specialist on activation
- Run in the same tokio task as the specialist
- Share the specialist's token allocation
- Report findings to `federation.relic.<name>.report`
- Accessible only through specialist context

Example (Ariel + Glass):

```
Specialist Loop (Ariel):
├── Check metabolic tokens
├── Invoke Glass (supervised relic)
│   ├── Glass runs within Ariel's task
│   ├── Glass consumes Ariel's token
│   ├── Glass collects visual/spatial findings
│   └── Publishes to federation.relic.glass.report
└── Ariel publishes own analysis to federation.specialist.ariel.report
```

## Metabolic Governance

### Token-Bucket System

**Global Expression Rate** controls cognitive throughput across all specialists:

```
expression_rate = 0.0 to 1.0

Each specialist's regeneration rate:
  regen_rate = (1000 / interval_ms) * expression_rate

For example, Ariel (interval_ms=20000):
  regen_rate = (1000 / 20000) * expression_rate
            = 0.05 * expression_rate tokens/sec
```

### Throttle States

| State | Expression Rate | Behavior |
|-------|-----------------|----------|
| **Normal** | 0.7-1.0 | All specialists execute at configured intervals |
| **Metabolic** | 0.3-0.7 | Reduced throughput; specialists still responsive |
| **Dormant** | 0.0-0.3 | Emergency mode; only critical tasks execute (e.g., Sentinel) |

### Adjusting Expression Rate

**Via NATS Control:**

```json
{
  "subject": "federation.control.set_expression_rate",
  "payload": {
    "rate": 0.5
  }
}
```

**Effect:** All specialists' metabolism recalibrated to 50% throughput immediately.

## NATS Federation Bus

### Published Topics

| Topic | Frequency | Payload |
|-------|-----------|---------|
| `federation.heartbeat` | Every 5s | `{repo: "Aaroneous", tokens: float, expression_rate: float, throttle_state: string}` |
| `federation.specialist.<name>.report` | Per-specialist interval | `{specialist: string, domain: string, findings: object, timestamp: number}` |
| `federation.relic.<name>.report` | Per-relic interval | `{relic: string, supervisor: string, findings: object, timestamp: number}` |
| `federation.user.<user_id>.activity` | On-demand | `{user_id: string, action: string, target: string, timestamp: number}` |

### Subscription Topics (Control)

| Topic | Expected Payload | Action |
|-------|------------------|--------|
| `federation.control.spawn_specialist` | `{name: string, activate: bool, user_id?: string}` | Spawn specialist task |
| `federation.control.halt_specialist` | `{name: string}` | Gracefully shutdown specialist |
| `federation.control.set_expression_rate` | `{rate: float}` | Adjust global expression_rate |
| `federation.control.recalibrate_specialist` | `{name: string, bias: {...}}` | Hot-patch specialist epigenetics |
| `federation.control.spawn_user` | `{username: string, role: string}` | Create user agent session |

## User Interaction Model

### User Agent Lifecycle

When a user connects to the system:

1. Aaroneous creates a UserAgent with unique user_id
2. Optionally binds to a preferred specialist (e.g., user prefers Ariel for UI design)
3. User receives read-only view of:
   - System health (expression_rate, throttle_state, specialist status)
   - Active specialist/relic reports
   - Personal interaction history
4. User can request specialist actions within their permission scope

### User Permissions Model

Users have role-based permissions:

| Role | Permissions |
|------|-------------|
| **Observer** | read, observe (no mutations) |
| **Operator** | read, observe, request_specialist_action |
| **Administrator** | read, observe, request_specialist_action, adjust_expression_rate, manage_users |

## Production Operations

### Startup Command

**Console Mode (debugging):**

```powershell
.\bin\a-run.exe --console
```

**Service Mode (production):**

```powershell
# Install service
.\bin\a-run.exe --install

# Start service
net start AaroneousARun

# Stop service
net stop AaroneousARun
```

### Monitoring

**Via Federation Heartbeat:**

```
Watch federation.heartbeat subject for:
- expression_rate (should be 1.0 in normal operation)
- tokens (should grow over time)
- throttle_state (should be "Normal")
```

**Via Specialist Reports:**

```
Monitor federation.specialist.*.report for:
- Execution count (should increase steadily)
- Error findings (should be empty/minimal)
- Domain-specific metrics
```

### Emergency Procedures

**If system is overloaded:**

1. Reduce expression_rate via NATS:
   ```json
   {"subject": "federation.control.set_expression_rate", "payload": {"rate": 0.5}}
   ```

2. Halt non-critical specialists:
   ```json
   {"subject": "federation.control.halt_specialist", "payload": {"name": "dionysus"}}
   ```

3. Keep Sentinel (security) and Forge (manufacturing) active

**If critical error detected:**

- Sentinel will publish escalated threat to `federation.specialist.argus.report`
- Administrator should review logs in `logs/` directory
- Consider full restart: `net stop AaroneousARun && net start AaroneousARun`

## Logging & Telemetry

All events are logged to `D:\Aaroneous\logs/`:

- `arun_core.log` – Kernel lifecycle events
- `specialist_<name>.log` – Per-specialist execution
- `enzyme_errors.log` – DLL/WASM enzyme failures
- `federation_bus.log` – NATS connectivity and message flow

## Configuration

### HOX Phenotype System

Each specialist/relic loads its immutable HOX preset:

**Specialist HOX Example (hox_specialist_ariel.json):**

```json
{
  "chromosome_id": "specialist-ariel-v1",
  "agent_type": "Specialist",
  "name": "Ariel",
  "domain": "UserInterface",
  "phenotype": {
    "enforced_enzymes": ["sensor_node", "tensor_forge"]
  },
  "epigenetics": {
    "cognitive_bias": {
      "analytical_depth": 65,
      "creative_variance": 95,
      "audit_strictness": 40
    },
    "persona": "Creative, intuitive, and empathetic...",
    "interval_ms": 20000
  }
}
```

### Modifying Cognitive Biases (Epigenetics)

To tune a specialist's reasoning without code changes:

1. Update `hox_specialist_<name>.json` epigenetics section
2. Send NATS control message:

```json
{
  "subject": "federation.control.recalibrate_specialist",
  "payload": {
    "name": "ariel",
    "bias": {
      "analytical_depth": 75,
      "creative_variance": 90,
      "audit_strictness": 45
    }
  }
}
```

3. Specialist immediately applies new bias without restart

## Glossary

- **Enzyme:** Compiled binary module (DLL/WASM) that performs specialized computation
- **HOX Map:** Configuration file defining agent phenotype, epigenetics, and identity
- **Phenotype:** Observable characteristics (domain, role, enforced enzymes)
- **Epigenetics:** Behavioral tuning without genetic change (cognitive biases, persona)
- **Specialist:** Interactive personification supervised by human or system direction
- **Relic:** Smart artifact supervised exclusively by a specialist
- **SharedMemorySynapse:** Zero-copy IPC buffer between kernel and enzymes
- **Token-Bucket:** Rate-limiting mechanism using virtual "tokens" consumed per execution
- **Expression Rate:** Global multiplier (0.0-1.0) affecting all specialists' metabolism

---

**Last Updated:** 2026-04-28  
**Schema Version:** 1.0  
**Aaroneous Version:** 0.1.0 (Epoch VI - Production Finalization)
