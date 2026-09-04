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

        current_states.iter().any(|s| self.accept_states.contains(s))
    }
}

/// Mealy State Machine emitting outputs during state transitions.
#[derive(Debug, Clone)]
pub struct MealyMachine<S: std::hash::Hash + Eq + Clone, I: std::hash::Hash + Eq + Clone, O: Clone> {
    pub current_state: S,
    pub transitions: HashMap<(S, I), (S, O)>,
}

impl<S: std::hash::Hash + Eq + Clone, I: std::hash::Hash + Eq + Clone, O: Clone> MealyMachine<S, I, O> {
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
        if let Some((next_state, output)) = self.transitions.get(&(self.current_state.clone(), input)) {
            self.current_state = next_state.clone();
            Some(output.clone())
        } else {
            None
        }
    }
}

/// Moore State Machine with output associated with current state.
#[derive(Debug, Clone)]
pub struct MooreMachine<S: std::hash::Hash + Eq + Clone, I: std::hash::Hash + Eq + Clone, O: Clone> {
    pub current_state: S,
    pub transitions: HashMap<(S, I), S>,
    pub outputs: HashMap<S, O>,
}

impl<S: std::hash::Hash + Eq + Clone, I: std::hash::Hash + Eq + Clone, O: Clone> MooreMachine<S, I, O> {
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
        assert!(!dfa.accepts(&[1, 0, 0]));    // 1 one -> odd -> reject
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
