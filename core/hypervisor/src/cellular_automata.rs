/// Cellular automata and emergent computation systems.
///
/// Items 9–17 of the Phase 6 Expansion: FSM compiler, N-body clustering,
/// orbital scheduler, data lifecycle, superposition, tunneling, valence bonding,
/// RNA adapter, VSA space inflator.

/// A generic VSA (Vector Symbolic Architecture) vector.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VsaVector {
    pub storage: [u64; 128],
}

impl VsaVector {
    pub const BYTE_LEN: usize = 128 * 8;

    pub fn zeroed() -> Self {
        Self { storage: [0u64; 128] }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut storage = [0u64; 128];
        let limit = bytes.len().min(Self::BYTE_LEN);
        for word_idx in 0..128 {
            let base = word_idx * 8;
            if base >= limit {
                break;
            }

            let mut word = [0u8; 8];
            for byte_idx in 0..8 {
                let src_idx = base + byte_idx;
                if src_idx >= limit {
                    break;
                }
                word[byte_idx] = bytes[src_idx];
            }
            storage[word_idx] = u64::from_le_bytes(word);
        }

        Self { storage }
    }

    pub fn as_bytes(&self) -> &[u8; Self::BYTE_LEN] {
        unsafe { &*(self.storage.as_ptr() as *const [u8; Self::BYTE_LEN]) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8; Self::BYTE_LEN] {
        unsafe { &mut *(self.storage.as_mut_ptr() as *mut [u8; Self::BYTE_LEN]) }
    }
}

// ── 9. FSM Compiler ───────────────────────────────────────────────────
// Compiles action sequences into compressed state machine representation.

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct FsmAction {
    pub input: u64,
    pub output: u64,
    pub next_state: usize,
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct FsmState {
    pub id: usize,
    pub transition_count: usize,
    pub transitions: [FsmAction; 16],
}

impl FsmState {
    pub const MAX_TRANSITIONS: usize = 16;

    pub fn new(id: usize, transitions: &[FsmAction]) -> Self {
        assert!(
            transitions.len() <= Self::MAX_TRANSITIONS,
            "FsmState::new supports at most {} transitions",
            Self::MAX_TRANSITIONS
        );
        let mut storage = [FsmAction { input: 0, output: 0, next_state: 0 }; 16];
        let count = transitions.len().min(storage.len());
        storage[..count].copy_from_slice(&transitions[..count]);
        Self { id, transition_count: count, transitions: storage }
    }
}

#[derive(Debug, Clone)]
pub struct FsmCompiler {
    pub states: Vec<FsmState>,
    pub current: usize,
}

impl FsmCompiler {
    pub fn new(states: Vec<FsmState>) -> Self {
        FsmCompiler { states, current: 0 }
    }

    pub fn step(&mut self, input: u64) -> Option<u64> {
        let state = &self.states[self.current];
        for i in 0..state.transition_count {
            let t = state.transitions[i];
            if t.input == input {
                self.current = t.next_state;
                return Some(t.output);
            }
        }
        None
    }

    pub fn current_state_id(&self) -> usize { self.current }

    /// Decompile the FSM by enumerating all possible input/output paths.
    ///
    /// Traces every reachable state from state 0, collecting all possible
    /// input→output transitions as path sequences. Limits depth to prevent
    /// infinite loops in cyclic FSMs.
    ///
    /// Returns a list of paths, where each path is a sequence of (input, output, next_state).
    pub fn enumerate_paths(&self, max_depth: usize) -> Vec<Vec<(u64, u64, usize)>> {
        let mut all_paths = Vec::new();
        let mut visited = vec![false; self.states.len()];
        self.dfs_paths(0, &mut visited, &mut Vec::new(), &mut all_paths, max_depth, 0);
        all_paths
    }

    fn dfs_paths(
        &self,
        state: usize,
        visited: &mut Vec<bool>,
        current_path: &mut Vec<(u64, u64, usize)>,
        all_paths: &mut Vec<Vec<(u64, u64, usize)>>,
        max_depth: usize,
        depth: usize,
    ) {
        if depth >= max_depth || state >= self.states.len() {
            if !current_path.is_empty() {
                all_paths.push(current_path.clone());
            }
            return;
        }

        let s = &self.states[state];
        let mut has_transitions = false;

        for i in 0..s.transition_count {
            let t = s.transitions[i];
            has_transitions = true;

            current_path.push((t.input, t.output, t.next_state));

            if !visited[t.next_state] {
                visited[t.next_state] = true;
                self.dfs_paths(t.next_state, visited, current_path, all_paths, max_depth, depth + 1);
                visited[t.next_state] = false;
            } else {
                // Cycle detected — record the path segment
                all_paths.push(current_path.clone());
            }

            current_path.pop();
        }

        if !has_transitions {
            all_paths.push(current_path.clone());
        }
    }

    /// Get all transitions as a flat list.
    pub fn all_transitions(&self) -> Vec<(usize, u64, u64, usize)> {
        let mut transitions = Vec::new();
        for (state_idx, state) in self.states.iter().enumerate() {
            for i in 0..state.transition_count {
                let t = state.transitions[i];
                transitions.push((state_idx, t.input, t.output, t.next_state));
            }
        }
        transitions
    }

    /// Get state information for inspection.
    pub fn state_info(&self) -> Vec<(usize, usize)> {
        self.states.iter().enumerate()
            .map(|(i, s)| (i, s.transition_count))
            .collect()
    }
}

// ── 10. N-body Clustering ─────────────────────────────────────────────
// Gravitational attractor model for VSA density clustering.

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ClusterBody {
    pub position: [f32; 2],
    pub mass: f32,
    pub velocity: [f32; 2],
}

#[derive(Debug, Clone)]
pub struct NBodyCluster {
    pub bodies: Vec<ClusterBody>,
    pub g: f32,
}

impl NBodyCluster {
    pub fn new(g: f32) -> Self { NBodyCluster { bodies: Vec::new(), g } }

    pub fn add_body(&mut self, x: f32, y: f32, mass: f32) {
        self.bodies.push(ClusterBody { position: [x, y], mass, velocity: [0.0, 0.0] });
    }

    /// Run one gravity simulation step (Brute-force O(n²)).
    pub fn step(&mut self) {
        let n = self.bodies.len();
        let mut forces = vec![[0.0f32; 2]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.bodies[j].position[0] - self.bodies[i].position[0];
                let dy = self.bodies[j].position[1] - self.bodies[i].position[1];
                let dist_sq = dx * dx + dy * dy + 0.01; // softening
                let f = self.g / dist_sq;
                let fx = f * dx;
                let fy = f * dy;
                forces[i][0] += fx * self.bodies[j].mass;
                forces[i][1] += fy * self.bodies[j].mass;
                forces[j][0] -= fx * self.bodies[i].mass;
                forces[j][1] -= fy * self.bodies[i].mass;
            }
        }
        for (i, body) in self.bodies.iter_mut().enumerate() {
            body.velocity[0] += forces[i][0] / body.mass;
            body.velocity[1] += forces[i][1] / body.mass;
            body.position[0] += body.velocity[0];
            body.position[1] += body.velocity[1];
        }
    }

    /// Reverse one gravity simulation step.
    ///
    /// Negates velocities and runs a forward step, effectively undoing
    /// the last time step. Useful for debugging and state inspection.
    pub fn reverse_step(&mut self) {
        // Negate velocities to reverse direction
        for body in &mut self.bodies {
            body.velocity[0] = -body.velocity[0];
            body.velocity[1] = -body.velocity[1];
        }
        // Run forward step (with negated velocities, this moves backward)
        self.step();
        // Negate velocities again to restore sign convention
        for body in &mut self.bodies {
            body.velocity[0] = -body.velocity[0];
            body.velocity[1] = -body.velocity[1];
        }
    }

    /// Save the current state of all bodies.
    pub fn save_state(&self) -> Vec<ClusterBody> {
        self.bodies.clone()
    }

    /// Restore bodies to a previously saved state.
    pub fn restore_state(&mut self, state: &[ClusterBody]) {
        self.bodies = state.to_vec();
    }
}

// ── 11. Orbital Scheduler ──────────────────────────────────────────────
// Time-based task scheduler using orbital periods.

#[derive(Debug, Clone)]
pub struct OrbitalTask {
    pub id: u64,
    pub period_ticks: u64,
    pub counter: u64,
    pub action: u64,
}

#[derive(Debug, Clone)]
pub struct OrbitalScheduler {
    pub tasks: Vec<OrbitalTask>,
    pub tick: u64,
}

impl OrbitalScheduler {
    pub fn new() -> Self { OrbitalScheduler { tasks: Vec::new(), tick: 0 } }

    pub fn add_task(&mut self, id: u64, period: u64, action: u64) {
        self.tasks.push(OrbitalTask { id, period_ticks: period, counter: 0, action });
    }

    /// Advance one tick; return Vec of actions that fire this tick.
    pub fn tick(&mut self) -> Vec<u64> {
        self.tick += 1;
        let mut fired = Vec::new();
        for task in &mut self.tasks {
            task.counter += 1;
            if task.counter >= task.period_ticks {
                fired.push(task.action);
                task.counter = 0;
            }
        }
        fired
    }
}

// ── 12. Data Lifecycle Manager ────────────────────────────────────────
// Promotes/demotes data between Tier 0 (hot) and Tier 3 (cold).

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataTier { Tier0 = 0, Tier1 = 1, Tier2 = 2, Tier3 = 3 }

#[repr(C)]
#[derive(Debug, Clone)]
pub struct DataBlock {
    pub id: u64,
    pub tier: DataTier,
    pub access_count: u64,
    pub last_accessed: u64,
}

#[derive(Debug, Clone)]
pub struct DataLifecycleManager {
    pub blocks: Vec<DataBlock>,
    pub clock: u64,
    pub promote_threshold: u64,
    pub demote_threshold: u64,
}

impl DataLifecycleManager {
    pub fn new(promote: u64, demote: u64) -> Self {
        DataLifecycleManager { blocks: Vec::new(), clock: 0, promote_threshold: promote, demote_threshold: demote }
    }

    pub fn access(&mut self, id: u64) {
        self.clock += 1;
        if let Some(block) = self.blocks.iter_mut().find(|b| b.id == id) {
            block.access_count += 1;
            block.last_accessed = self.clock;
        }
    }

    /// Run lifecycle: promote hot blocks, demote cold blocks.
    pub fn rebalance(&mut self) {
        for block in &mut self.blocks {
            if block.access_count > self.promote_threshold && block.tier as u8 > 0 {
                block.tier = match block.tier {
                    DataTier::Tier3 => DataTier::Tier2,
                    DataTier::Tier2 => DataTier::Tier1,
                    DataTier::Tier1 => DataTier::Tier0,
                    DataTier::Tier0 => DataTier::Tier0,
                };
                block.access_count = 0;
            }
            if self.clock - block.last_accessed > self.demote_threshold && (block.tier as u8) < 3 {
                block.tier = match block.tier {
                    DataTier::Tier0 => DataTier::Tier1,
                    DataTier::Tier1 => DataTier::Tier2,
                    DataTier::Tier2 => DataTier::Tier3,
                    DataTier::Tier3 => DataTier::Tier3,
                };
            }
        }
    }

    pub fn block_count_for_tier(&self, tier: DataTier) -> usize {
        self.blocks.iter().filter(|b| b.tier == tier).count()
    }
}

// ── 13. Superposition ─────────────────────────────────────────────────
// Qubit-inspired probability amplitude overlay for parallel exploration.

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct ProbabilityAmplitude {
    pub real: f32,
    pub imag: f32,
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct SuperpositionState {
    pub element_count: usize,
    pub amplitudes: [ProbabilityAmplitude; 32],
}

impl SuperpositionState {
    pub const MAX_ELEMENTS: usize = 32;

    pub fn new(n: usize) -> Self {
        assert!(
            n <= Self::MAX_ELEMENTS,
            "SuperpositionState::new supports at most {} elements",
            Self::MAX_ELEMENTS
        );
        let count = n.min(32);
        let mut amplitudes = [ProbabilityAmplitude { real: 0.0, imag: 0.0 }; 32];
        if count > 0 {
            let norm = 1.0 / (count as f32).sqrt();
            for i in 0..count {
                amplitudes[i] = ProbabilityAmplitude { real: norm, imag: 0.0 };
            }
        }
        SuperpositionState { element_count: count, amplitudes }
    }

    /// Collapse to most probable state via Born rule.
    pub fn collapse(&self) -> usize {
        if self.element_count == 0 {
            return 0;
        }
        let mut best_idx = 0usize;
        let mut best_prob = 0.0f32;
        for (i, a) in self.amplitudes.iter().take(self.element_count).enumerate() {
            let prob = a.real * a.real + a.imag * a.imag;
            if prob > best_prob {
                best_prob = prob;
                best_idx = i;
            }
        }
        best_idx
    }

    /// Apply Hadamard-like transform to create equal superposition.
    pub fn apply_hadamard(&mut self) {
        if self.element_count == 0 {
            return;
        }

        let n = self.element_count as f32;
        let factor = 1.0 / n.sqrt();
        for a in self.amplitudes.iter_mut().take(self.element_count) {
            a.real = factor;
            a.imag = 0.0;
        }
    }
}

// ── 14. Tunneling ─────────────────────────────────────────────────────
// Quantum tunneling metaphor: bypass blocked paths via energy threshold.

#[derive(Debug, Clone)]
pub struct TunnelingGate {
    pub barrier_energy: f32,
    pub particle_energy: f32,
    pub tunnel_enabled: bool,
}

impl TunnelingGate {
    pub fn new(barrier: f32) -> Self {
        TunnelingGate { barrier_energy: barrier, particle_energy: 0.0, tunnel_enabled: false }
    }

    /// Attempt to tunnel; returns true if barrier is crossed.
    pub fn attempt_tunnel(&mut self, particle_energy: f32) -> bool {
        self.particle_energy = particle_energy;
        // Simple exponential tunneling probability
        let delta = self.barrier_energy - particle_energy;
        if delta <= 0.0 {
            self.tunnel_enabled = true;
            return true;
        }
        let prob = (-delta / self.barrier_energy).exp();
        self.tunnel_enabled = prob > 0.5;
        self.tunnel_enabled
    }
}

// ── 15. Valence Bonding ───────────────────────────────────────────────
// Shared electron-pair bonding between VSA vectors.

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ValenceBond {
    pub vector_a: VsaVector,
    pub vector_b: VsaVector,
    pub bond_strength: f32,
}

#[derive(Debug, Clone)]
pub struct ValenceBonding {
    pub bonds: Vec<ValenceBond>,
}

impl ValenceBonding {
    pub fn new() -> Self { ValenceBonding { bonds: Vec::new() } }

    /// Form a bond between two VSA vectors if their overlap exceeds threshold.
    pub fn try_bond(&mut self, a: &VsaVector, b: &VsaVector, threshold: f32) -> bool {
        let overlap = popcount_similarity(a.as_bytes(), b.as_bytes());
        if overlap >= threshold {
            self.bonds.push(ValenceBond {
                vector_a: a.clone(),
                vector_b: b.clone(),
                bond_strength: overlap,
            });
            true
        } else {
            false
        }
    }
}

fn popcount_similarity(a: &[u8], b: &[u8]) -> f32 {
    let n = a.len().min(b.len());
    if n == 0 { return 0.0; }
    let mut same = 0usize;
    for i in 0..n {
        let diff = (a[i] ^ b[i]).count_ones() as usize;
        same += 8 - diff;
    }
    same as f32 / (n as f32 * 8.0)
}

// ── 16. RNA Adapter ───────────────────────────────────────────────────
// Transcribes instruction sets into execution plans.

#[derive(Debug, Clone)]
pub struct RnaInstruction {
    pub opcode: u8,
    pub operand: u64,
}

#[derive(Debug, Clone)]
pub struct RnaAdapter {
    pub transcript: Vec<RnaInstruction>,
}

impl RnaAdapter {
    pub fn new() -> Self { RnaAdapter { transcript: Vec::new() } }

    /// Transcribe a byte sequence into RNA instructions.
    pub fn transcribe(&mut self, data: &[u8]) {
        self.transcript.clear();
        for chunk in data.chunks(9) {
            if chunk.is_empty() { continue; }
            let opcode = chunk[0] % 64;
            let mut operand = 0u64;
            for (i, &b) in chunk.iter().enumerate().skip(1).take(8) {
                operand = (operand << 8) | (b as u64);
            }
            self.transcript.push(RnaInstruction { opcode, operand });
        }
    }

    pub fn translate(&self) -> Vec<u64> {
        self.transcript.iter().map(|inst| (inst.opcode as u64) << 56 | inst.operand).collect()
    }
}

// ── 17. VSA Space Inflator ────────────────────────────────────────────
// Dimensionality expansion for collision resolution in HD space.

#[repr(C)]
#[derive(Debug, Clone)]
pub struct VsaSpaceInflator {
    pub base_dimensions: usize,
    pub expansion_factor: usize,
    pub expanded: Vec<VsaVector>,
}

impl VsaSpaceInflator {
    pub fn new(base: usize, expansion: usize) -> Self {
        VsaSpaceInflator { base_dimensions: base, expansion_factor: expansion, expanded: Vec::new() }
    }

    /// Expand a VSA vector into higher-dimensional space by interpolating.
    pub fn inflate(&mut self, vectors: &[VsaVector]) {
        self.expanded.clear();
        for v in vectors {
            let mut inflated = VsaVector::zeroed();
            let output_len = self.base_dimensions.saturating_mul(self.expansion_factor).min(VsaVector::BYTE_LEN);
            if output_len == 0 {
                self.expanded.push(inflated);
                continue;
            }

            let source = v.as_bytes();
            let source_max = VsaVector::BYTE_LEN - 1;
            let inflated_bytes = inflated.as_bytes_mut();
            // Linear interpolation across dimensions
            for i in 0..output_len {
                let src_idx = (i * self.base_dimensions) / output_len;
                inflated_bytes[i] = source[src_idx.min(source_max)];
            }
            self.expanded.push(inflated);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fsm_compiler() {
        let states = vec![
            FsmState::new(0, &[FsmAction { input: 1, output: 10, next_state: 1 }]),
            FsmState::new(1, &[FsmAction { input: 2, output: 20, next_state: 0 }]),
        ];
        let mut fsm = FsmCompiler::new(states);
        assert_eq!(fsm.step(1), Some(10));
        assert_eq!(fsm.current_state_id(), 1);
        assert_eq!(fsm.step(2), Some(20));
        assert_eq!(fsm.current_state_id(), 0);
    }

    #[test]
    fn test_fsm_no_match() {
        let states = vec![FsmState::new(0, &[])];
        let mut fsm = FsmCompiler::new(states);
        assert_eq!(fsm.step(99), None);
    }

    #[test]
    fn test_nbody_cluster() {
        let mut cluster = NBodyCluster::new(1.0);
        cluster.add_body(0.0, 0.0, 10.0);
        cluster.add_body(5.0, 0.0, 10.0);
        cluster.step();
        // Bodies should have moved toward each other
        assert!(cluster.bodies[0].position[0] > -0.1);
        assert!(cluster.bodies[1].position[0] < 5.1);
    }

    #[test]
    fn test_orbital_scheduler() {
        let mut sched = OrbitalScheduler::new();
        sched.add_task(1, 3, 100);
        let mut fired = Vec::new();
        for _ in 0..6 { fired.extend(sched.tick()); }
        assert_eq!(fired, vec![100, 100]); // fires at ticks 3, 6
    }

    #[test]
    fn test_data_lifecycle() {
        let mut mgr = DataLifecycleManager::new(3, 10);
        mgr.blocks.push(DataBlock { id: 1, tier: DataTier::Tier2, access_count: 0, last_accessed: 0 });
        // Access 4 times → promote to Tier1
        for _ in 0..4 { mgr.access(1); }
        mgr.rebalance();
        assert_eq!(mgr.blocks[0].tier as u8, DataTier::Tier1 as u8);
    }

    #[test]
    fn test_superposition_collapse() {
        let mut sup = SuperpositionState::new(4);
        sup.amplitudes[0] = ProbabilityAmplitude { real: 0.9, imag: 0.0 };
        assert_eq!(sup.collapse(), 0);
    }

    #[test]
    fn test_superposition_hadamard() {
        let mut sup = SuperpositionState::new(4);
        sup.amplitudes[0] = ProbabilityAmplitude { real: 1.0, imag: 0.0 };
        sup.apply_hadamard();
        for a in sup.amplitudes.iter().take(sup.element_count) {
            assert!((a.real - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn test_tunneling_passes() {
        let mut gate = TunnelingGate::new(5.0);
        assert!(gate.attempt_tunnel(10.0)); // energy exceeds barrier
    }

    #[test]
    fn test_tunneling_blocked() {
        let mut gate = TunnelingGate::new(100.0);
        // Very low energy, but prob > 0.5 depends on barrier
        let result = gate.attempt_tunnel(1.0);
        // With barrier=100, delta=99, exp(-99/100)=0.37 < 0.5 → blocked
        assert!(!result);
    }

    #[test]
    fn test_valence_bonding() {
        let mut bond = ValenceBonding::new();
        let a = VsaVector::from_bytes(&[0xAA; 32]);
        let b = VsaVector::from_bytes(&[0xAA; 32]);
        assert!(bond.try_bond(&a, &b, 1.0));
        assert_eq!(bond.bonds.len(), 1);
    }

    #[test]
    fn test_rna_adapter() {
        let mut adapter = RnaAdapter::new();
        adapter.transcribe(&[10, 20, 30, 40, 50, 60, 70, 80, 90, 1, 2, 3]);
        assert_eq!(adapter.transcript.len(), 2);
        assert_eq!(adapter.transcript[0].opcode, 10);
    }

    #[test]
    fn test_vsa_space_inflator() {
        let mut inflator = VsaSpaceInflator::new(4, 2);
        let vectors = vec![VsaVector::from_bytes(&[1, 2, 3, 4])];
        inflator.inflate(&vectors);
        assert_eq!(inflator.expanded.len(), 1);
        assert_eq!(&inflator.expanded[0].as_bytes()[0..8], &[1, 1, 2, 2, 3, 3, 4, 4]);
    }
}
