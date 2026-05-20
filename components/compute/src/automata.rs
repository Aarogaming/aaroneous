/// Automata Theory primitives (FSM, Pushdown, Regex).
/// Used for AST parsing, de-zombification logic, and protocol validation.

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

    pub fn run(&self, input: &[usize]) -> bool {
        let mut state = 0;
        for &sym in input {
            state = self.transitions[state][sym % self.transitions[0].len()];
        }
        self.accept.get(state).copied().unwrap_or(false)
    }
}

/// Regex-like pattern matcher (simplified DFA compilation)
pub fn dfa_match(pattern: &[u8], text: &[u8]) -> bool {
    // Simplified: exact match for now, extensible to NFA->DFA
    if pattern.len() > text.len() { return false; }
    text.windows(pattern.len()).any(|window| window == pattern)
}
