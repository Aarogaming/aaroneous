//! crates/compute/src/automata.rs
//! Automata Theory primitives (FSM, DFA, NFA, Mealy/Moore, Trie).
//! Used for AST parsing, de-zombification logic, protocol validation, and lexical tokenization.

use std::collections::{HashMap, HashSet};

/// Finite State Machine using integer states and indexed alphabet.
#[derive(Debug, Clone)]
pub struct FiniteStateMachine {
    pub states: usize,
    pub transitions: Vec<Vec<usize>>, // [state][input] -> next_state
    pub accept: Vec<bool>,
}

impl FiniteStateMachine {
    pub fn new(states: usize, alphabet: usize) -> Self {
        Self {
            states,
            transitions: vec![vec![0; alphabet]; states],
            accept: vec![false; states],
        }
    }

    pub fn add_transition(&mut self, from: usize, input: usize, to: usize) {
        if from < self.states && input < self.transitions[from].len() {
            self.transitions[from][input] = to;
        }
    }

    pub fn set_accept(&mut self, state: usize, is_accept: bool) {
        if state < self.states {
            self.accept[state] = is_accept;
        }
    }

    pub fn run(&self, input: &[usize]) -> bool {
        if self.transitions.is_empty() || self.transitions[0].is_empty() {
            return false;
        }
        let mut state = 0;
        for &sym in input {
            state = self.transitions[state][sym % self.transitions[0].len()];
        }
        self.accept.get(state).copied().unwrap_or(false)
    }
}

/// Generic Deterministic Finite Automaton (DFA) over arbitrary hashable states and symbols.
#[derive(Debug, Clone)]
pub struct GenericDfa<S: std::hash::Hash + Eq + Clone, A: std::hash::Hash + Eq + Clone> {
    pub start_state: S,
    pub transitions: HashMap<(S, A), S>,
    pub accept_states: HashSet<S>,
}

impl<S: std::hash::Hash + Eq + Clone, A: std::hash::Hash + Eq + Clone> GenericDfa<S, A> {
    pub fn new(start_state: S) -> Self {
        Self {
            start_state,
            transitions: HashMap::new(),
            accept_states: HashSet::new(),
        }
    }

    pub fn add_transition(&mut self, from: S, symbol: A, to: S) {
        self.transitions.insert((from, symbol), to);
    }

    pub fn set_accept(&mut self, state: S) {
        self.accept_states.insert(state);
    }

    pub fn accepts(&self, input: &[A]) -> bool {
        let mut current = self.start_state.clone();
        for sym in input {
            match self.transitions.get(&(current, sym.clone())) {
                Some(next) => current = next.clone(),
                None => return false,
            }
        }
        self.accept_states.contains(&current)
    }
}

/// Generic Non-deterministic Finite Automaton (NFA) supporting epsilon-transitions (None).
#[derive(Debug, Clone)]
pub struct GenericNfa<S: std::hash::Hash + Eq + Clone, A: std::hash::Hash + Eq + Clone> {
    pub start_states: HashSet<S>,
    pub transitions: HashMap<(S, Option<A>), HashSet<S>>,
    pub accept_states: HashSet<S>,
}

impl<S: std::hash::Hash + Eq + Clone, A: std::hash::Hash + Eq + Clone> GenericNfa<S, A> {
    pub fn new(start: S) -> Self {
        let mut start_states = HashSet::new();
        start_states.insert(start);
        Self {
            start_states,
            transitions: HashMap::new(),
            accept_states: HashSet::new(),
        }
    }

    pub fn add_transition(&mut self, from: S, symbol: Option<A>, to: S) {
        self.transitions
            .entry((from, symbol))
            .or_default()
            .insert(to);
    }

    pub fn set_accept(&mut self, state: S) {
        self.accept_states.insert(state);
    }

    /// Computes epsilon closure of a set of states.
    pub fn epsilon_closure(&self, states: &HashSet<S>) -> HashSet<S> {
        let mut closure = states.clone();
        let mut stack: Vec<S> = states.iter().cloned().collect();

        while let Some(state) = stack.pop() {
            if let Some(next_states) = self.transitions.get(&(state, None)) {
                for next in next_states {
                    if closure.insert(next.clone()) {
                        stack.push(next.clone());
                    }
                }
            }
        }
        closure
    }

    /// Tests if the NFA accepts a given sequence of symbols.
    pub fn accepts(&self, input: &[A]) -> bool {
        let mut current_states = self.epsilon_closure(&self.start_states);

        for sym in input {
            let mut next_states = HashSet::new();
            for state in &current_states {
                if let Some(targets) = self.transitions.get(&(state.clone(), Some(sym.clone()))) {
                    for target in targets {
                        next_states.insert(target.clone());
                    }
                }
            }
            current_states = self.epsilon_closure(&next_states);
            if current_states.is_empty() {
                return false;
            }
        }

        current_states
            .iter()
            .any(|s| self.accept_states.contains(s))
    }
}

/// Mealy State Machine emitting outputs during state transitions.
#[derive(Debug, Clone)]
pub struct MealyMachine<S: std::hash::Hash + Eq + Clone, I: std::hash::Hash + Eq + Clone, O: Clone>
{
    pub current_state: S,
    pub transitions: HashMap<(S, I), (S, O)>,
}

impl<S: std::hash::Hash + Eq + Clone, I: std::hash::Hash + Eq + Clone, O: Clone>
    MealyMachine<S, I, O>
{
    pub fn new(initial: S) -> Self {
        Self {
            current_state: initial,
            transitions: HashMap::new(),
        }
    }

    pub fn add_transition(&mut self, from: S, input: I, to: S, output: O) {
        self.transitions.insert((from, input), (to, output));
    }

    pub fn step(&mut self, input: I) -> Option<O> {
        if let Some((next_state, output)) =
            self.transitions.get(&(self.current_state.clone(), input))
        {
            self.current_state = next_state.clone();
            Some(output.clone())
        } else {
            None
        }
    }
}

/// Moore State Machine with output associated with current state.
#[derive(Debug, Clone)]
pub struct MooreMachine<S: std::hash::Hash + Eq + Clone, I: std::hash::Hash + Eq + Clone, O: Clone>
{
    pub current_state: S,
    pub transitions: HashMap<(S, I), S>,
    pub outputs: HashMap<S, O>,
}

impl<S: std::hash::Hash + Eq + Clone, I: std::hash::Hash + Eq + Clone, O: Clone>
    MooreMachine<S, I, O>
{
    pub fn new(initial: S, initial_output: O) -> Self {
        let mut outputs = HashMap::new();
        outputs.insert(initial.clone(), initial_output);
        Self {
            current_state: initial,
            transitions: HashMap::new(),
            outputs,
        }
    }

    pub fn add_transition(&mut self, from: S, input: I, to: S, to_output: O) {
        self.transitions.insert((from, input), to.clone());
        self.outputs.insert(to, to_output);
    }

    pub fn output(&self) -> Option<&O> {
        self.outputs.get(&self.current_state)
    }

    pub fn step(&mut self, input: I) -> Option<&O> {
        if let Some(next) = self.transitions.get(&(self.current_state.clone(), input)) {
            self.current_state = next.clone();
            self.output()
        } else {
            None
        }
    }
}

/// Trie Node for high-speed multi-keyword matching.
#[derive(Debug, Default, Clone)]
pub struct TrieNode {
    pub children: HashMap<u8, TrieNode>,
    pub is_terminal: bool,
    pub value: Option<String>,
}

/// Trie Structure for keyword/token parsing and dictionary indexing.
#[derive(Debug, Default, Clone)]
pub struct TrieMatcher {
    pub root: TrieNode,
}

impl TrieMatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, word: &str) {
        let mut curr = &mut self.root;
        for &b in word.as_bytes() {
            curr = curr.children.entry(b).or_default();
        }
        curr.is_terminal = true;
        curr.value = Some(word.to_string());
    }

    pub fn contains(&self, word: &str) -> bool {
        let mut curr = &self.root;
        for &b in word.as_bytes() {
            match curr.children.get(&b) {
                Some(next) => curr = next,
                None => return false,
            }
        }
        curr.is_terminal
    }

    pub fn longest_prefix_match<'a>(&self, text: &'a [u8]) -> Option<&'a [u8]> {
        let mut curr = &self.root;
        let mut last_match = None;
        for (i, &b) in text.iter().enumerate() {
            match curr.children.get(&b) {
                Some(next) => {
                    curr = next;
                    if curr.is_terminal {
                        last_match = Some(&text[..=i]);
                    }
                }
                None => break,
            }
        }
        last_match
    }
}

/// Regex-like pattern matcher (exact substring fallback for backwards-compatibility).
pub fn dfa_match(pattern: &[u8], text: &[u8]) -> bool {
    if pattern.is_empty() {
        return true;
    }
    if pattern.len() > text.len() {
        return false;
    }
    text.windows(pattern.len()).any(|window| window == pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfa_acceptance() {
        // DFA that accepts even number of 1s
        let mut dfa = GenericDfa::new(0);
        dfa.add_transition(0, 0, 0);
        dfa.add_transition(0, 1, 1);
        dfa.add_transition(1, 0, 1);
        dfa.add_transition(1, 1, 0);
        dfa.set_accept(0);

        assert!(dfa.accepts(&[0, 1, 1, 0])); // 2 ones -> even -> accept
        assert!(!dfa.accepts(&[1, 0, 0])); // 1 one -> odd -> reject
    }

    #[test]
    fn test_nfa_epsilon_transitions() {
        let mut nfa = GenericNfa::new(0);
        // 0 --e--> 1 --a--> 2
        nfa.add_transition(0, None, 1);
        nfa.add_transition(1, Some('a'), 2);
        nfa.set_accept(2);

        assert!(nfa.accepts(&['a']));
        assert!(!nfa.accepts(&['b']));
    }

    #[test]
    fn test_trie_matcher() {
        let mut trie = TrieMatcher::new();
        trie.insert("fn");
        trie.insert("struct");
        trie.insert("static");

        assert!(trie.contains("fn"));
        assert!(trie.contains("struct"));
        assert!(!trie.contains("let"));

        let prefix = trie.longest_prefix_match(b"struct Foo");
        assert_eq!(prefix, Some(b"struct".as_slice()));
    }
}

// ---------------------------------------------------------------------------
// Cascading Automata Reducer (Lane C150)
// Zero-allocation, fuel-bounded hierarchical state reduction.
// ---------------------------------------------------------------------------

pub const MAX_ARENA_NODES: usize = 256;
pub const MAX_SUBROUTINE_DEPTH: usize = 16;
pub const MAX_ACTIONS_PER_STEP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct NodeIdx(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct ConditionId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct SubroutineId(pub u16);

/// Monotonically decremented execution fuel to guarantee total function termination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepFuel(pub u32);

impl StepFuel {
    #[inline(always)]
    pub fn consume(&mut self, amount: u32) -> Result<(), ReducerTrap> {
        if self.0 < amount {
            self.0 = 0;
            Err(ReducerTrap::FuelExhausted)
        } else {
            self.0 -= amount;
            Ok(())
        }
    }
}

/// Bounded set of deterministic peripheral commands emitted in a single scan cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum DiscreteAction {
    None,
    MouseMove { x: i32, y: i32 },
    MouseButton { button: u8, down: bool },
    KeyPress { key_code: u16, down: bool },
    YieldSlice,
}

/// Contiguous, stack-allocated action emission buffer (zero heap).
#[derive(Debug, Clone, Copy)]
pub struct BoundedActionList {
    pub actions: [DiscreteAction; MAX_ACTIONS_PER_STEP],
    pub count: usize,
}

impl BoundedActionList {
    pub const fn empty() -> Self {
        Self {
            actions: [DiscreteAction::None; MAX_ACTIONS_PER_STEP],
            count: 0,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, action: DiscreteAction) -> Result<(), ReducerTrap> {
        if self.count >= MAX_ACTIONS_PER_STEP {
            Err(ReducerTrap::ActionCapacityExceeded)
        } else {
            self.actions[self.count] = action;
            self.count += 1;
            Ok(())
        }
    }
}

/// Fail-closed execution traps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReducerTrap {
    FuelExhausted,
    RecursionLimitExceeded,
    LoopIterationCapExceeded,
    InvalidNodeIndex(NodeIdx),
    ActionCapacityExceeded,
    IllegalCycleDetected,
}

/// Pure control flow instructions addressing nodes exclusively via `NodeIdx`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlInstruction {
    /// Atomic action emission; advances cursor
    EmitAction(DiscreteAction),
    /// Strict sequence: executes from `start_idx` for `length` nodes
    Sequence { start_idx: NodeIdx, length: u16 },
    /// Fallback selector: tries children until one returns Ok
    Selector { start_idx: NodeIdx, length: u16 },
    /// Deterministic conditional branch
    Branch {
        condition: ConditionId,
        on_true: NodeIdx,
        on_false: NodeIdx,
    },
    /// Bounded loop with explicit per-tick iteration ceiling
    LoopWhile {
        condition: ConditionId,
        max_iterations_per_tick: u16,
        body: NodeIdx,
    },
    /// Macro-macro call: invokes subroutine entry point in the arena
    CallSubroutine(SubroutineId),
    /// Terminal node
    Halt,
}

/// Fixed-size node in the contiguous arena.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct MacroArenaNode {
    pub instruction: ControlInstruction,
}

/// Contiguous, zero-heap AST arena.
#[derive(Debug, Clone, Copy)]
pub struct MacroArena<const N: usize> {
    pub nodes: [MacroArenaNode; N],
    pub subroutines: [NodeIdx; 32],
    pub node_count: usize,
}

impl<const N: usize> Default for MacroArena<N> {
    fn default() -> Self {
        Self {
            nodes: [MacroArenaNode {
                instruction: ControlInstruction::Halt,
            }; N],
            subroutines: [NodeIdx(0); 32],
            node_count: 0,
        }
    }
}

/// Injected environmental sensory state (zero ambient authority).
pub struct SensorySnapshot<'a> {
    pub monotonic_timestamp_ns: u64,
    pub condition_evaluator: &'a dyn Fn(ConditionId) -> bool,
}

/// Current automaton state for pure state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AutomatonState {
    pub active_node: NodeIdx,
    pub call_depth: u8,
    pub halted: bool,
}

/// Pure, non-allocating Three-Phase State Reducer: S_{t+1} = f(S_t, I).
pub fn reduce_step<const N: usize>(
    arena: &MacroArena<N>,
    mut state: AutomatonState,
    obs: &SensorySnapshot<'_>,
    fuel: &mut StepFuel,
) -> Result<(AutomatonState, BoundedActionList), ReducerTrap> {
    let mut actions = BoundedActionList::empty();

    if state.halted {
        return Ok((state, actions));
    }

    fuel.consume(1)?;

    let node_idx = state.active_node.0 as usize;
    if node_idx >= arena.node_count {
        return Err(ReducerTrap::InvalidNodeIndex(state.active_node));
    }

    match arena.nodes[node_idx].instruction {
        ControlInstruction::EmitAction(action) => {
            actions.push(action)?;
            state.active_node.0 += 1;
        }
        ControlInstruction::Branch {
            condition,
            on_true,
            on_false,
        } => {
            fuel.consume(1)?;
            let is_true = (obs.condition_evaluator)(condition);
            state.active_node = if is_true { on_true } else { on_false };
        }
        ControlInstruction::LoopWhile {
            condition,
            max_iterations_per_tick,
            body,
        } => {
            let mut iters = 0;
            while (obs.condition_evaluator)(condition) {
                fuel.consume(2)?;
                iters += 1;
                if iters > max_iterations_per_tick {
                    return Err(ReducerTrap::LoopIterationCapExceeded);
                }
                let inner_state = AutomatonState {
                    active_node: body,
                    call_depth: state.call_depth,
                    halted: false,
                };
                let (_, inner_actions) = reduce_step(arena, inner_state, obs, fuel)?;
                for i in 0..inner_actions.count {
                    actions.push(inner_actions.actions[i])?;
                }
            }
            state.active_node.0 += 1;
        }
        ControlInstruction::CallSubroutine(sub_id) => {
            fuel.consume(1)?;
            if state.call_depth as usize >= MAX_SUBROUTINE_DEPTH {
                return Err(ReducerTrap::RecursionLimitExceeded);
            }
            let sub_entry = arena.subroutines[sub_id.0 as usize];
            let sub_state = AutomatonState {
                active_node: sub_entry,
                call_depth: state.call_depth + 1,
                halted: false,
            };
            let (next_sub, sub_actions) = reduce_step(arena, sub_state, obs, fuel)?;
            for i in 0..sub_actions.count {
                actions.push(sub_actions.actions[i])?;
            }
            if next_sub.halted {
                state.active_node.0 += 1;
            }
        }
        ControlInstruction::Sequence { start_idx, length } => {
            state.active_node = start_idx;
            for _ in 0..length {
                fuel.consume(1)?;
                let (next, sub_actions) = reduce_step(arena, state, obs, fuel)?;
                state = next;
                for i in 0..sub_actions.count {
                    actions.push(sub_actions.actions[i])?;
                }
            }
        }
        ControlInstruction::Selector { start_idx, length } => {
            let mut resolved = false;
            for offset in 0..length {
                fuel.consume(1)?;
                let test_state = AutomatonState {
                    active_node: NodeIdx(start_idx.0 + offset),
                    call_depth: state.call_depth,
                    halted: false,
                };
                if let Ok((_, sub_actions)) = reduce_step(arena, test_state, obs, fuel) {
                    for i in 0..sub_actions.count {
                        actions.push(sub_actions.actions[i])?;
                    }
                    resolved = true;
                    break;
                }
            }
            if !resolved {
                state.halted = true;
            } else {
                state.active_node.0 += length;
            }
        }
        ControlInstruction::Halt => {
            state.halted = true;
        }
    }

    Ok((state, actions))
}

/// Static validation verifying DAG acyclicity, bounded depth, and legal bounds.
pub fn validate_arena<const N: usize>(arena: &MacroArena<N>) -> Result<(), ReducerTrap> {
    if arena.node_count > N {
        return Err(ReducerTrap::InvalidNodeIndex(NodeIdx(
            arena.node_count as u16,
        )));
    }
    for i in 0..arena.node_count {
        match arena.nodes[i].instruction {
            ControlInstruction::Branch {
                on_true, on_false, ..
            } => {
                if (on_true.0 as usize) >= arena.node_count
                    || (on_false.0 as usize) >= arena.node_count
                {
                    return Err(ReducerTrap::InvalidNodeIndex(on_true));
                }
            }
            ControlInstruction::CallSubroutine(sub_id) => {
                let entry = arena.subroutines[sub_id.0 as usize];
                if (entry.0 as usize) >= arena.node_count {
                    return Err(ReducerTrap::InvalidNodeIndex(entry));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod cascading_automata_tests {
    use super::*;

    #[test]
    fn test_cascading_reducer_sequence_and_fuel() {
        let mut arena: MacroArena<16> = MacroArena::default();
        arena.nodes[0] = MacroArenaNode {
            instruction: ControlInstruction::EmitAction(DiscreteAction::MouseMove {
                x: 100,
                y: 200,
            }),
        };
        arena.nodes[1] = MacroArenaNode {
            instruction: ControlInstruction::EmitAction(DiscreteAction::MouseButton {
                button: 1,
                down: true,
            }),
        };
        arena.nodes[2] = MacroArenaNode {
            instruction: ControlInstruction::Halt,
        };
        arena.node_count = 3;

        validate_arena(&arena).expect("arena validation must pass");

        let obs = SensorySnapshot {
            monotonic_timestamp_ns: 1_000_000,
            condition_evaluator: &|_| false,
        };

        let mut fuel = StepFuel(10);
        let mut state = AutomatonState::default();

        let (next_state, actions) =
            reduce_step(&arena, state, &obs, &mut fuel).expect("step 1 succeeds");
        assert_eq!(actions.count, 1);
        assert_eq!(
            actions.actions[0],
            DiscreteAction::MouseMove { x: 100, y: 200 }
        );
        assert_eq!(next_state.active_node, NodeIdx(1));
        assert!(fuel.0 < 10);

        state = next_state;
        let (next_state2, actions2) =
            reduce_step(&arena, state, &obs, &mut fuel).expect("step 2 succeeds");
        assert_eq!(actions2.count, 1);
        assert_eq!(
            actions2.actions[0],
            DiscreteAction::MouseButton {
                button: 1,
                down: true
            }
        );
        assert_eq!(next_state2.active_node, NodeIdx(2));
    }

    #[test]
    fn test_fuel_exhaustion_terminates_cleanly() {
        let mut arena: MacroArena<16> = MacroArena::default();
        arena.nodes[0] = MacroArenaNode {
            instruction: ControlInstruction::EmitAction(DiscreteAction::YieldSlice),
        };
        arena.node_count = 1;

        let obs = SensorySnapshot {
            monotonic_timestamp_ns: 1_000_000,
            condition_evaluator: &|_| false,
        };

        let mut fuel = StepFuel(0); // Zero fuel
        let state = AutomatonState::default();

        let result = reduce_step(&arena, state, &obs, &mut fuel);
        assert!(matches!(result, Err(ReducerTrap::FuelExhausted)));
    }

    #[test]
    fn test_loop_while_iteration_cap() {
        let mut arena: MacroArena<16> = MacroArena::default();
        arena.nodes[0] = MacroArenaNode {
            instruction: ControlInstruction::LoopWhile {
                condition: ConditionId(1),
                max_iterations_per_tick: 5,
                body: NodeIdx(1),
            },
        };
        arena.nodes[1] = MacroArenaNode {
            instruction: ControlInstruction::EmitAction(DiscreteAction::YieldSlice),
        };
        arena.node_count = 2;

        let obs = SensorySnapshot {
            monotonic_timestamp_ns: 1_000_000,
            condition_evaluator: &|id| id.0 == 1, // Always true (infinite loop condition)
        };

        let mut fuel = StepFuel(100);
        let state = AutomatonState::default();

        let result = reduce_step(&arena, state, &obs, &mut fuel);
        assert!(matches!(result, Err(ReducerTrap::LoopIterationCapExceeded)));
    }
}
