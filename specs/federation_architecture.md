# Federation Architecture Specification v1.0

## Overview

The Federation module implements a distributed specialist hive architecture where each specialist operates as an independent agent with its own GGUF model, coordinated by a Sentinel orchestrator without becoming a bottleneck.

## Core Principles

1. **Independent Specialists**: Each specialist maintains isolated state and model weights
2. **Bidirectional Intent Flow**: Top-down task delegation + bottom-up proposal negotiation
3. **Conflict-Free Resolution**: Built-in arbitration for competing specialist actions
4. **Sovereign Packages**: Self-contained deployment units with manifest verification

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        Federation Layer                          │
├─────────────────────────────────────────────────────────────────┤
│  Sentinel (2GB) ─── Conflict Resolution ─── Resource Allocation │
│         │                                              │        │
│         ▼                                              ▼        │
│  ┌──────────────┐                              ┌──────────────┐│
│  │ Visionary    │                              │ Omnipresent  ││
│  │ (Design Gen) │                              │ (P2P Sync)   ││
│  └──────────────┘                              └──────────────┘│
│         │                                              │        │
│         ▼                                              ▼        │
│  ┌──────────────┐                              ┌──────────────┐│
│  │ Symbiotic    │                              │ Phygital     ││
│  │ (Biometrics) │                              │ (AR/VR)      ││
│  └──────────────┘                              └──────────────┘│
│         │                                              │        │
│         └──────────────┬───────────────────────────────┘        │
│                        ▼                                        │
│                  ┌──────────────┐                               │
│                  │  Archivist   │ ← ArtifactRegistry             │
│                  │ (500MB)      │ ← Persistence Layer            │
│                  └──────────────┘                               │
└─────────────────────────────────────────────────────────────────┘

IPC Bus (SignalBridge) → Zero-copy message passing
P2P Mesh (Iroh QUIC)   → Optional federated coordination
```

## Core Components

### 1. Specialist Interface

```rust
pub trait Specialist: Send + Sync {
    fn id(&self) -> SpecialistId;
    fn model_path(&self) -> &Path;
    fn propose_action(&self, context: &SpecialistContext) -> Proposal;
    fn execute(&self, proposal: &Proposal) -> ExecutionResult;
    fn learn(&mut self, experience: &LearningStateSnapshot);
}
```

### 2. Sentinel Orchestrator

```rust
pub struct Sentinel {
    config: SentinelConfig,
    specialists: Arc<SpecialistRegistry>,
    conflict_arbitrator: Arc<ConflictArbitrator>,
}

impl Sentinel {
    pub fn new(config: SentinelConfig) -> Result<Self>;
    
    pub fn dispatch_intent(&self, intent: Intent) -> Decision;
    
    pub fn resolve_conflict(&self, conflicts: &[Conflict]) -> ArbitrationResult;
}
```

### 3. Sovereign Package Format

```rust
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[rkyv(derive(Debug))]
pub struct SovereignManifest {
    pub version: u32,
    pub specialists: Vec<SpecialistId>,
    pub capabilities: CapabilityMask,
    pub checksum: u64,  // CRC64 for integrity
}

pub fn export_sovereign(specialists: &[Arc<dyn Specialist>]) -> Result<Vec<u8>>;
pub fn import_sovereign(data: &[u8]) -> ImportResult;
```

## Memory Model

| Component | Size | Purpose |
|-----------|------|---------|
| Sentinel | 2GB | Orchestrator state, conflict resolution buffers |
| Visionary | 1GB | Design generation model weights |
| Omnipresent | 1GB | P2P sync state, connection tracking |
| Symbiotic | 500MB | Biometric polling buffers |
| Phygital | 1GB | Depth processing, landmark caching |
| Archivist | 500MB | ArtifactRegistry snapshots |

**Total**: ~6GB peak memory (configurable per deployment)

## Communication Protocol

### Message Types

```rust
pub enum SpecialistMessage {
    Intent(Intent),
    Proposal(Proposal),
    ExecutionResult(ExecutionResult),
    LearningSnapshot(LearningStateSnapshot),
    ConflictReport(Conflict),
    HealthCheck,
}
```

### Zero-Copy Pipeline

1. **Producer** creates `SpecialistMessage` in stack memory
2. **SignalBridge** wraps message in lock-free SPMC ring buffer
3. **Consumer** deserializes via `rkyv::Deserialize` (zero-copy)
4. **WGPU compute shader** processes tensor payloads if present

## Deployment Patterns

### Single-Hive Mode

```bash
a_run federation --specialists visionary,symbiotic,archivist \
  --sentinel-model sentinel-2b.gguf \
  --ipc-bind 127.0.0.1:8001
```

### Multi-Hive Federation

```bash
# Node A
a_run federation --node-id node-a \
  --p2p-address /ip4/192.168.1.100/udp/8001/quic \
  --specialists visionary,omnipresent

# Node B  
a_run federation --node-id node-b \
  --p2p-address /ip4/192.168.1.101/udp/8001/quic \
  --specialists symbiotic,phygital,archivist
```

## Security Model

- **Capability Mask**: Each specialist granted explicit permissions (read/write/storage/network)
- **Sandboxed Execution**: WASM micro-VM for untrusted code paths
- **JIT Audit Gate**: Machine code scanned for privileged instructions before execution
- **TLS Encryption**: Inter-node communication encrypted via Iroh QUIC

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Intent-to-Decision Latency | < 50ms | Sentinel dispatch time |
| Specialist Proposal Round-Trip | < 10ms | IPC bus throughput |
| Conflict Resolution Time | < 25ms | Arbitrator decision |
| Memory Footprint | ≤ 6GB | RSS at peak load |

## Implementation Status

### Complete (Production-Ready)
- ✅ `specialist.rs` - Core specialist implementation
- ✅ `sentinel.rs` - Orchestrator logic
- ✅ `conflict_resolution.rs` - Arbitration engine
- ✅ `communication.rs` - IPC message bus
- ✅ `sovereign_package.rs` - Package format

### Partial (Needs Integration)
- ⚠️ `forge.rs` - GGUF crystallization (needs SiForge integration)
- ⚠️ `model_registry.rs` - Specialist registration (needs ArtifactRegistry sync)
- ⚠️ `learn_persist.rs` - Learning state persistence

### Speculative (Future Work)
- 🔮 `multi_hive.rs` - P2P federation across nodes
- 🔮 `enterprise.rs` - Audit/compliance logging
- 🔮 `biometric.rs` - Hardware polling integration

## Migration Path

1. **Phase 1**: Wire all federation modules into `core/hypervisor/src/lib.rs`
2. **Phase 2**: Replace simulated stubs in `delta_orchestrator.rs` with real KV cache extraction
3. **Phase 3**: Integrate SiForge for GGUF model crystallization
4. **Phase 4**: Enable P2P mesh via Iroh QUIC transport

## References

- `.si` Format: `crates/si_format/`
- SiForge Builder: `crates/compute/src/si_forge.rs`
- IPC Bus: `crates/ipc_bus/`
- Artifact Registry: `adaptation_engine/artifact_registry.rs`
