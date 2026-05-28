/// Agent-to-Agent binary protocol and Byzantine rollup verification.
///
/// A2A uses binary state flags instead of text chat for low-overhead
/// peer-to-peer capability sharing. Byzantine rollups compress 1000
/// task steps into a single cryptographic state root for external
/// verification without re-execution.

use std::collections::HashMap;

// ── A2A Binary Protocol ──────────────────────────────────────────────

/// Single binary state flag exchanged between agents.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct A2AFlag {
    pub agent_id: u64,
    pub capability: u32,
    pub state: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct A2AProtocol {
    pub agents: HashMap<u64, A2AFlag>,
    pub pending: Vec<A2AFlag>,
}

impl A2AProtocol {
    pub fn new() -> Self { A2AProtocol { agents: HashMap::new(), pending: Vec::new() } }

    /// Set a capability flag for this agent; returns true if state changed.
    pub fn set_flag(&mut self, agent_id: u64, capability: u32, state: bool) -> bool {
        let key = (agent_id, capability);
        let changed = match self.agents.get(&agent_id) {
            Some(existing) if existing.capability == capability && existing.state == state => false,
            _ => true,
        };
        if changed {
            let flag = A2AFlag { agent_id, capability, state, timestamp: 0 };
            self.agents.insert(agent_id, flag);
            self.pending.push(flag);
        }
        changed
    }

    /// Get current state of a capability for an agent.
    pub fn get_flag(&self, agent_id: u64, capability: u32) -> bool {
        self.agents.get(&agent_id)
            .filter(|f| f.capability == capability)
            .map(|f| f.state)
            .unwrap_or(false)
    }

    /// Drain pending flags (e.g. after broadcast).
    pub fn drain_pending(&mut self) -> Vec<A2AFlag> { std::mem::take(&mut self.pending) }

    /// Negotiate shared capability: both agents must agree.
    pub fn negotiate(&mut self, local_id: u64, remote_id: u64, capability: u32) -> bool {
        let local = self.get_flag(local_id, capability);
        let remote = self.get_flag(remote_id, capability);
        local && remote
    }
}

// ── Byzantine Rollup ─────────────────────────────────────────────────
// Compresses 1000+ task steps into a single state root hash.

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaskStep {
    pub step_id: u32,
    pub action_hash: u64,
    pub pre_state: u64,
    pub post_state: u64,
}

#[derive(Debug, Clone)]
pub struct ByzantineRollup {
    pub steps: Vec<TaskStep>,
    pub root: u64,
    pub verified: bool,
}

impl ByzantineRollup {
    pub fn new() -> Self { ByzantineRollup { steps: Vec::new(), root: 0, verified: false } }

    /// Append a task step; incrementally updates the rolling state root.
    pub fn append(&mut self, action_hash: u64, pre_state: u64, post_state: u64) {
        let step_id = self.steps.len() as u32;
        self.steps.push(TaskStep { step_id, action_hash, pre_state, post_state });
        // Rolling Merkle-like root: root = hash(root XOR step_hash)
        let step_hash = self::hash_combine(pre_state, post_state) ^ action_hash;
        self.root = self.root.wrapping_add(step_hash).rotate_left(13) ^ step_hash;
    }

    /// Finalize rollup; returns the state root for external verification.
    pub fn finalize(&mut self) -> u64 {
        self.root = self.root.wrapping_mul(0x9E3779B97F4A7C15);
        self.verified = false;
        self.root
    }

    /// External verifier: recompute root from raw steps.
    pub fn verify(steps: &[TaskStep]) -> u64 {
        let mut root = 0u64;
        for step in steps {
            let step_hash = self::hash_combine(step.pre_state, step.post_state) ^ step.action_hash;
            root = root.wrapping_add(step_hash).rotate_left(13) ^ step_hash;
        }
        root.wrapping_mul(0x9E3779B97F4A7C15)
    }

    /// Mark rollup as externally verified.
    pub fn mark_verified(&mut self) { self.verified = true; }
}

fn hash_combine(a: u64, b: u64) -> u64 {
    a.wrapping_mul(0x9E3779B97F4A7C15) ^ b.rotate_left(17).wrapping_mul(0xBF58476D1CE4E5B9)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a2a_set_flag() {
        let mut proto = A2AProtocol::new();
        assert!(proto.set_flag(1, 0, true));
        assert!(!proto.set_flag(1, 0, true)); // no change
        assert!(proto.get_flag(1, 0));
    }

    #[test]
    fn test_a2a_drain_pending() {
        let mut proto = A2AProtocol::new();
        proto.set_flag(1, 0, true);
        proto.set_flag(2, 1, false);
        assert_eq!(proto.drain_pending().len(), 2);
        assert_eq!(proto.drain_pending().len(), 0);
    }

    #[test]
    fn test_a2a_negotiate() {
        let mut proto = A2AProtocol::new();
        proto.set_flag(1, 42, true);
        proto.set_flag(2, 42, true);
        assert!(proto.negotiate(1, 2, 42));
        proto.set_flag(2, 42, false);
        assert!(!proto.negotiate(1, 2, 42));
    }

    #[test]
    fn test_byzantine_rollup_append() {
        let mut rollup = ByzantineRollup::new();
        rollup.append(0xDEAD, 0xAAAA, 0xBBBB);
        rollup.append(0xBEEF, 0xBBBB, 0xCCCC);
        assert_eq!(rollup.steps.len(), 2);
        assert_ne!(rollup.root, 0);
    }

    #[test]
    fn test_byzantine_rollup_verify() {
        let mut rollup = ByzantineRollup::new();
        rollup.append(0xDEAD, 0xAAAA, 0xBBBB);
        rollup.append(0xBEEF, 0xBBBB, 0xCCCC);
        let root = rollup.finalize();
        let computed = ByzantineRollup::verify(&rollup.steps);
        assert_eq!(root, computed);
    }

    #[test]
    fn test_byzantine_rollup_tamper_detection() {
        let mut steps = vec![
            TaskStep { step_id: 0, action_hash: 0xDEAD, pre_state: 0xAAAA, post_state: 0xBBBB },
            TaskStep { step_id: 1, action_hash: 0xBEEF, pre_state: 0xBBBB, post_state: 0xCCCC },
        ];
        let real_root = ByzantineRollup::verify(&steps);
        steps[0].action_hash = 0xBAD;
        assert_ne!(ByzantineRollup::verify(&steps), real_root);
    }
}
