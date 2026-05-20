# Phase 6B: Raft Consensus Engine

**Objective**: Implement distributed consensus for multi-node federation with strong consistency guarantees.

**Timeline**: 16-20 hours estimated  
**Target**: 450+ unit tests (45-50 new tests)  
**Success Criteria**: Multi-node election, log replication, atomic mutations, snapshots

---

## Architecture Overview

### Core Components

```
┌─────────────────────────────────────────────────────┐
│         Raft Consensus Engine                       │
├─────────────────────────────────────────────────────┤
│  RaftNode (per instance)                            │
│  ├── RaftState (Follower/Candidate/Leader)         │
│  ├── LogEntry (immutable, append-only)             │
│  ├── PersistentState (currentTerm, votedFor)       │
│  └── VolatileState (commitIndex, lastApplied)      │
│                                                     │
│  RaftLog (append-only event log)                   │
│  ├── entries: Vec<LogEntry>                        │
│  ├── snapshots: Vec<Snapshot>                      │
│  └── lastIncludedIndex/Term                        │
│                                                     │
│  RaftPeer (remote node connection)                 │
│  ├── nextIndex (next log index to send)            │
│  ├── matchIndex (highest replicated index)         │
│  └── last_heartbeat (for failure detection)        │
│                                                     │
│  RaftEngine (coordinator)                          │
│  ├── nodes: HashMap<NodeId, RaftNode>             │
│  ├── election_timeout (150-300ms)                  │
│  ├── heartbeat_interval (50ms)                     │
│  └── tick() [periodic state machine]               │
└─────────────────────────────────────────────────────┘
```

### State Transitions

```
         (start)
            │
            ▼
    ┌──────────────┐
    │  Follower    │  (initial state)
    │  (receives   │  - Votes for leaders
    │   RPCs)      │  - Replicates log entries
    └──────────────┘
            ▲
            │ (election timeout / leader fails)
            │
            ▼
    ┌──────────────┐
    │  Candidate   │  (election phase)
    │  (votes for  │  - Increments term
    │   self)      │  - Sends vote requests
    └──────────────┘
            ▲
            │ (receives votes from quorum)
            │
            ▼
    ┌──────────────┐
    │   Leader     │  (stable state)
    │   (replicates│  - Sends heartbeats
    │   log)       │  - Handles mutations
    └──────────────┘
```

---

## Implementation Plan

### Phase 6B.1: Core Raft Types & State Machine

**Files to Create**:
- `src/raft_consensus/mod.rs` - Module definition
- `src/raft_consensus/types.rs` - RPC types, state enums
- `src/raft_consensus/log.rs` - Append-only log implementation
- `src/raft_consensus/node.rs` - RaftNode state machine
- `src/raft_consensus/engine.rs` - Multi-node coordinator

**Key Types**:

```rust
/// LogEntry - immutable record with:
/// - index: u64
/// - term: u64  
/// - data: FederationEvent (from event_log)
/// - snapshot_index: Option<u64> (for compaction)

/// RaftState - node's current role:
/// - Follower { leader_id: Option<NodeId>, last_heartbeat: Instant }
/// - Candidate { votes_received: u32 }
/// - Leader { peer_states: HashMap<NodeId, PeerState> }

/// PersistentState (survives restarts):
/// - currentTerm: u64
/// - votedFor: Option<NodeId>
/// - log: Vec<LogEntry>

/// VolatileState (reset on restart):
/// - commitIndex: u64
/// - lastApplied: u64

/// AppendEntriesRPC - replication message:
/// - term: u64
/// - leader_id: NodeId
/// - prev_log_index: u64
/// - prev_log_term: u64
/// - entries: Vec<LogEntry>
/// - leader_commit: u64

/// RequestVoteRPC - election message:
/// - term: u64
/// - candidate_id: NodeId
/// - last_log_index: u64
/// - last_log_term: u64

/// ApplyCommand - mutation to apply:
/// - data: FederationEvent
/// - client_id: String (for dedup)
```

**Estimated Effort**: 6-8 hours
**Tests**: 20-25 (type tests, serialization, state transitions)

---

### Phase 6B.2: Leader Election

**Files to Enhance**:
- `src/raft_consensus/engine.rs` - add election logic

**Algorithm**:

```
On Election Timeout (Follower → Candidate):
1. Increment currentTerm
2. Vote for self
3. Reset election timeout
4. Send RequestVoteRPC to all other nodes
5. Wait for responses:
   - If receive votes from majority → become Leader
   - If receive AppendEntriesRPC from leader with term ≥ currentTerm → remain Follower
   - If election timeout again → restart election (new term)

Leader Heartbeat (periodic):
1. Send AppendEntriesRPC to all followers (empty if no new entries)
2. If follower doesn't respond, retry with backoff
3. On quorum acknowledgment → update commitIndex
```

**Key Safety Rules**:
- Don't vote twice in same term
- Only vote if candidate's log is at least as up-to-date
- Leadership term must be highest seen
- Once elected, can only lose via restart or follower becoming candidate

**Estimated Effort**: 4-5 hours
**Tests**: 15-20 (election scenarios, term updates, vote safety)

---

### Phase 6B.3: Log Replication

**Files to Enhance**:
- `src/raft_consensus/log.rs` - add replication support
- `src/raft_consensus/engine.rs` - add replication loop

**Algorithm**:

```
On Leader Receives Command (mutation request):
1. Append to own log
2. Send AppendEntriesRPC to all followers
3. Wait for acknowledgments:
   - If majority ACKs → mark as committed
   - If majority ACKs → apply to state machine
   - If majority ACKs → return success to client

On Follower Receives AppendEntriesRPC:
1. Check term (reject if less than currentTerm)
2. Check prev_log_index/prev_log_term (log consistency)
3. If entries provided:
   - Delete any conflicting entries
   - Append new entries
   - Persist to disk
4. If leader_commit > commitIndex:
   - Update commitIndex to min(leader_commit, last_log_index)
   - Apply newly committed entries
5. Return success/failure
```

**Consistency Mechanism (Critical!)**:
- Leader tracks nextIndex (next log index to send to each follower)
- On AppendEntriesRPC rejection: decrement nextIndex and retry
- This ensures all followers eventually have same prefix of log
- Only entries at leader's commitIndex are applied

**Estimated Effort**: 5-6 hours  
**Tests**: 20-25 (replication scenarios, conflict resolution, consistency)

---

### Phase 6B.4: Atomic Mutations & Quorum

**Files to Create**:
- `src/raft_consensus/mutations.rs` - mutation handling

**Implementation**:

```rust
/// Quorum calculation:
fn is_quorum(votes: usize, total_nodes: usize) -> bool {
    votes > total_nodes / 2
}

/// Atomic Mutation Flow:
1. Client sends mutation to leader via HTTP endpoint
2. Leader appends to own log immediately (not durably applied)
3. Leader sends AppendEntriesRPC to followers
4. Once majority acknowledges:
   - Mark entry as "committed" (safe to apply)
   - Apply to state machine
   - Return success to client
5. Followers apply when notified of commitIndex update

/// Safety Guarantees:
- Never apply same mutation twice (client_id + seq tracking)
- Never lose applied mutations (persistence)
- Strong consistency (leader is single source of truth for committed entries)
```

**Estimated Effort**: 3-4 hours
**Tests**: 15-18 (quorum calculations, mutation ordering, deduplication)

---

### Phase 6B.5: Snapshots & Log Compaction

**Files to Create**:
- `src/raft_consensus/snapshot.rs` - snapshot management

**Why Snapshots?**:
- Prevent log from growing unbounded
- Reduce startup time (don't replay all entries)
- Clean recovery after crashes

**Algorithm**:

```rust
/// Snapshot Structure:
pub struct Snapshot {
    pub index: u64,           // log index of last included entry
    pub term: u64,            // term of last included entry
    pub state_data: Vec<u8>,  // serialized state (all applied entries up to index)
    pub created_at: DateTime<Utc>,
}

/// Snapshotting Trigger (on leader):
1. If log size > 1GB or entry count > 100,000:
   - Serialize current state machine
   - Create snapshot at commitIndex
   - Delete all log entries before snapshot
   - Keep snapshots for last 3 commits (for recovery)

/// Slow Follower Recovery:
1. If follower's log is far behind (prev_log_index < lastIncludedIndex):
   - Leader sends InstallSnapshotRPC
   - Follower replaces entire state with snapshot
   - Follower resumes replication from lastIncludedIndex+1
```

**Estimated Effort**: 4-5 hours
**Tests**: 12-15 (snapshot creation, installation, recovery)

---

### Phase 6B.6: Integration & Testing

**Files to Enhance**:
- `src/mcp_service/service.rs` - integrate with MCP
- Add tests throughout

**Integration Points**:
```
HTTP Request (OpenCode/VS Code)
    ↓
McpService::call_capability()
    ↓
If mutation → RaftEngine::propose_mutation()
    ↓
Leader applies to log
    ↓
Broadcast to followers
    ↓
On quorum ACK → apply to state machine
    ↓
Return result to client
```

**Testing Strategy**:
- Unit tests for each component (500+ lines → ~50 tests)
- Integration tests (multi-node scenarios)
- Fault injection tests (node failures, network partitions)
- Consistency verification (all nodes reach same state)

**Estimated Effort**: 3-4 hours
**Tests**: 20-30 (integration, fault scenarios, consistency)

---

## File Structure

```
src/raft_consensus/
├── mod.rs                 (module definition, exports)
├── types.rs              (RPC types, enums, constants)
├── log.rs                (append-only log, entry management)
├── node.rs               (RaftNode state machine)
├── engine.rs             (RaftEngine coordinator, election, replication)
├── mutations.rs          (mutation handling, quorum, dedup)
├── snapshot.rs           (snapshot creation, installation, recovery)
└── tests/
    ├── test_types.rs     (serialization, validation)
    ├── test_election.rs  (leader election scenarios)
    ├── test_replication.rs (log replication, consistency)
    ├── test_mutations.rs (atomic mutations, quorum)
    ├── test_snapshots.rs (snapshot creation, recovery)
    └── test_integration.rs (multi-node scenarios, faults)
```

---

## Success Metrics

### Functionality
- ✅ Single leader elected from 3+ nodes
- ✅ Followers replicate log entries from leader
- ✅ Mutations become durable after quorum confirmation
- ✅ All nodes eventually reach same committed state
- ✅ New leader elected if current leader fails
- ✅ Snapshots reduce log size

### Performance
- Election time: <300ms
- Replication latency: <50ms per heartbeat
- Snapshot creation: <100ms for 1GB state
- Mutation apply: <10ms after quorum

### Reliability
- Tolerates F failures with 2F+1 nodes
- Never loses committed mutations
- Never applies non-committed mutations
- Recovers from snapshot on crash

### Testing
- 450+ total tests (50+ new for Raft)
- All tests passing
- 100% code coverage on critical paths

---

## Estimated Timeline

| Phase | Duration | Tests | Status |
|-------|----------|-------|--------|
| 6B.1: Core Types | 6-8h | 20-25 | Pending |
| 6B.2: Election | 4-5h | 15-20 | Pending |
| 6B.3: Replication | 5-6h | 20-25 | Pending |
| 6B.4: Mutations | 3-4h | 15-18 | Pending |
| 6B.5: Snapshots | 4-5h | 12-15 | Pending |
| 6B.6: Testing & Integration | 3-4h | 20-30 | Pending |
| **TOTAL** | **25-32h** | **102-133** | **Pending** |

**Realistic Expectation**: 16-20 hours focused work for 45-50 new tests

---

## Risk Mitigation

### Known Challenges
1. **Split Brain** (multiple leaders) - Mitigated by term numbers + persistence
2. **Log Divergence** - Mitigated by leader's nextIndex backtracking
3. **State Loss** - Mitigated by persistent storage + snapshots
4. **Slow Followers** - Mitigated by snapshot installation

### Testing Approach
- Chaos testing: Kill random nodes at random times
- Partition testing: Network splits
- Byzantine testing: Corrupted messages (not implemented - trust network)
- Scale testing: 3, 5, 7 node clusters

---

## References

**Raft Paper** (Ongaro & Ousterhout, 2014):
- Clear semantics for distributed consensus
- Proven correctness for leader election
- Log replication guarantees
- Snapshot & log compaction

**Key Invariants**:
- Election Safety: At most one leader per term
- Leader Append-Only: Leaders never overwrite/delete log entries
- Log Matching: If two entries have same index/term, they're identical
- Leader Completeness: Leader contains all committed entries
- State Machine Safety: If entry applied, it was committed
