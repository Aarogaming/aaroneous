/// Dynamic execution systems: speculative parallel branches and
/// asynchronous DAG task compilation.

use std::collections::{HashMap, VecDeque};

// ── Dynamic Execution Tree Speculation ───────────────────────────────
// Runs low-weight parallel alternate branches in WASM worker slots;
// discards failed timelines the millisecond screen state shifts.

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeculativeBranch {
    pub id: u32,
    pub worker_slot: u32,
    pub action_hash: u64,
    pub expected_state: u64,
    pub active: bool,
    pub discarded: bool,
}

#[derive(Debug, Clone)]
pub struct SpeculativeExecutor {
    pub branches: Vec<SpeculativeBranch>,
    pub max_workers: u32,
    pub next_id: u32,
}

impl SpeculativeExecutor {
    pub fn new(max_workers: u32) -> Self {
        SpeculativeExecutor { branches: Vec::new(), max_workers, next_id: 0 }
    }

    /// Fork a new speculative branch if a worker slot is available.
    pub fn fork(&mut self, action_hash: u64, expected_state: u64) -> Option<u32> {
        let active_count = self.branches.iter().filter(|b| b.active && !b.discarded).count() as u32;
        if active_count >= self.max_workers { return None; }
        let id = self.next_id;
        self.next_id += 1;
        self.branches.push(SpeculativeBranch {
            id, worker_slot: active_count, action_hash, expected_state, active: true, discarded: false,
        });
        Some(id)
    }

    /// Check actual screen state against all active branches.
    /// Returns IDs of branches whose expected state matches (winners).
    pub fn check_state(&mut self, actual_state: u64) -> Vec<u32> {
        let mut winners = Vec::new();
        for branch in &mut self.branches {
            if !branch.active || branch.discarded { continue; }
            if branch.expected_state == actual_state {
                winners.push(branch.id);
            } else {
                branch.discarded = true;
            }
        }
        winners
    }

    /// Discard a specific branch.
    pub fn discard(&mut self, id: u32) {
        if let Some(branch) = self.branches.iter_mut().find(|b| b.id == id) {
            branch.discarded = true;
        }
    }

    pub fn active_count(&self) -> usize {
        self.branches.iter().filter(|b| b.active && !b.discarded).count()
    }
}

// ── Asynchronous DAG Task Compilation ────────────────────────────────
// Converts tasks into a topological DAG; runs independent tasks in
// parallel while freezing dependent steps until parents finish.

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DAGTask {
    pub id: u32,
    pub weight: f32,
    pub completed: bool,
    pub failed: bool,
}

#[derive(Debug, Clone)]
pub struct DAGScheduler {
    pub tasks: HashMap<u32, DAGTask>,
    /// adjacency: parent → children
    pub children: HashMap<u32, Vec<u32>>,
    /// reverse: child → parents
    pub parents: HashMap<u32, Vec<u32>>,
    pub ready: VecDeque<u32>,
    pub in_degree: HashMap<u32, usize>,
}

impl DAGScheduler {
    pub fn new() -> Self {
        DAGScheduler {
            tasks: HashMap::new(),
            children: HashMap::new(),
            parents: HashMap::new(),
            ready: VecDeque::new(),
            in_degree: HashMap::new(),
        }
    }

    /// Add a task node.
    pub fn add_task(&mut self, id: u32, weight: f32) {
        self.tasks.insert(id, DAGTask { id, weight, completed: false, failed: false });
        self.children.entry(id).or_default();
        self.parents.entry(id).or_default();
        self.in_degree.entry(id).or_insert(0);
    }

    /// Add dependency edge: parent must complete before child.
    pub fn add_dependency(&mut self, parent: u32, child: u32) {
        self.children.entry(parent).or_default().push(child);
        self.parents.entry(child).or_default().push(parent);
        *self.in_degree.entry(child).or_insert(0) += 1;
    }

    /// Build the DAG: populate ready queue with zero-in-degree tasks.
    pub fn build(&mut self) {
        self.ready.clear();
        let ids: Vec<u32> = self.tasks.keys().copied().collect();
        for &id in &ids {
            let deg = self.parents.get(&id).map(|p| p.len()).unwrap_or(0);
            self.in_degree.insert(id, deg);
            if deg == 0 {
                self.ready.push_back(id);
            }
        }
    }

    /// Pop next ready task (returns None if nothing ready).
    pub fn next_ready(&mut self) -> Option<u32> { self.ready.pop_front() }

    /// Mark a task as completed; updates dependents to unblock them.
    pub fn complete(&mut self, id: u32) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.completed = true;
        }
        if let Some(kids) = self.children.get(&id) {
            for &child in kids {
                if let Some(deg) = self.in_degree.get_mut(&child) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        self.ready.push_back(child);
                    }
                }
            }
        }
    }

    /// Mark a task as failed.
    pub fn fail(&mut self, id: u32) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.failed = true;
        }
    }

    /// Returns true if all tasks are completed.
    pub fn all_completed(&self) -> bool {
        self.tasks.values().all(|t| t.completed || t.failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speculative_fork() {
        let mut exec = SpeculativeExecutor::new(4);
        let id = exec.fork(0xDEAD, 0xAAAA);
        assert!(id.is_some());
        assert_eq!(exec.active_count(), 1);
    }

    #[test]
    fn test_speculative_max_workers() {
        let mut exec = SpeculativeExecutor::new(2);
        exec.fork(0x1, 0xA);
        exec.fork(0x2, 0xB);
        assert!(exec.fork(0x3, 0xC).is_none()); // should be rejected
    }

    #[test]
    fn test_speculative_check_state() {
        let mut exec = SpeculativeExecutor::new(4);
        exec.fork(0xDEAD, 0xAAAA);
        exec.fork(0xBEEF, 0xBBBB);
        let winners = exec.check_state(0xAAAA);
        assert_eq!(winners.len(), 1);
        assert_eq!(exec.active_count(), 1); // only the winning branch stays active
    }

    #[test]
    fn test_dag_scheduler_basic() {
        let mut dag = DAGScheduler::new();
        dag.add_task(1, 1.0);
        dag.add_task(2, 1.0);
        dag.add_task(3, 1.0);
        dag.add_dependency(1, 2);
        dag.add_dependency(2, 3);
        dag.build();
        assert_eq!(dag.next_ready(), Some(1));
        assert_eq!(dag.next_ready(), None); // 2 blocked by 1
        dag.complete(1);
        assert_eq!(dag.next_ready(), Some(2));
        dag.complete(2);
        assert_eq!(dag.next_ready(), Some(3));
        dag.complete(3);
        assert!(dag.all_completed());
    }

    #[test]
    fn test_dag_scheduler_parallel() {
        let mut dag = DAGScheduler::new();
        dag.add_task(1, 1.0);
        dag.add_task(2, 1.0);
        dag.add_task(3, 1.0);
        dag.add_dependency(1, 3);
        dag.add_dependency(2, 3);
        dag.build();
        let tasks: Vec<_> = std::iter::from_fn(|| dag.next_ready()).collect();
        assert_eq!(tasks.len(), 2);
        assert!(tasks.contains(&1));
        assert!(tasks.contains(&2));
    }

    #[test]
    fn test_dag_scheduler_fail() {
        let mut dag = DAGScheduler::new();
        dag.add_task(1, 1.0);
        dag.build();
        dag.fail(1);
        assert!(dag.all_completed());
    }
}
