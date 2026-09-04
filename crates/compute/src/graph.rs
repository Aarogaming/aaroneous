//! crates/compute/src/graph.rs
//! Graph Theory, Network Centrality, and Spectral Analysis primitives.
//! Used for constellation mapping, module dependency ordering, routing, and community clustering.

use std::collections::{BinaryHeap, VecDeque};
use std::cmp::Ordering;

/// Computes degree centrality from adjacency matrix (flattened row-major).
pub fn degree_centrality(adj: &[f64], n: usize) -> Vec<f64> {
    if n == 0 || adj.len() < n * n {
        return vec![0.0; n];
    }
    let mut centrality = vec![0.0; n];
    for i in 0..n {
        for j in 0..n {
            centrality[i] += adj[i * n + j];
        }
        centrality[i] /= (n - 1).max(1) as f64;
    }
    centrality
}

/// Spectral clustering approximation (power iteration for dominant eigenvector).
pub fn spectral_cluster(adj: &[f64], n: usize, iterations: usize) -> Vec<f64> {
    if n == 0 || adj.len() < n * n {
        return vec![];
    }
    let mut eigenvector = vec![1.0 / (n as f64).sqrt(); n];
    for _ in 0..iterations {
        let mut next = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                next[i] += adj[i * n + j] * eigenvector[j];
            }
        }
        let norm = next.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        if norm > 0.0 {
            eigenvector = next.iter().map(|x| x / norm).collect();
        }
    }
    eigenvector
}

/// Computes PageRank scores for nodes in an adjacency matrix using the power iteration method.
pub fn page_rank(adj: &[f64], n: usize, damping: f64, iterations: usize) -> Vec<f64> {
    if n == 0 || adj.len() < n * n {
        return vec![];
    }

    let d = damping.clamp(0.0, 1.0);
    let mut out_degrees = vec![0.0; n];
    for i in 0..n {
        for j in 0..n {
            out_degrees[i] += adj[i * n + j];
        }
    }

    let mut rank = vec![1.0 / n as f64; n];
    let teleport = (1.0 - d) / n as f64;

    for _ in 0..iterations {
        let mut next_rank = vec![teleport; n];
        for i in 0..n {
            if out_degrees[i] > 0.0 {
                let share = (d * rank[i]) / out_degrees[i];
                for j in 0..n {
                    if adj[i * n + j] > 0.0 {
                        next_rank[j] += share * adj[i * n + j];
                    }
                }
            } else {
                // Dangling node distribution
                let dangling_share = (d * rank[i]) / n as f64;
                for val in next_rank.iter_mut().take(n) {
                    *val += dangling_share;
                }
            }
        }
        rank = next_rank;
    }

    rank
}

/// Dijkstra node for min-heap priority queue
#[derive(Copy, Clone, PartialEq)]
struct State {
    cost: f64,
    position: usize,
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice the reversal of comparison for min-heap
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Dijkstra's algorithm for single-source shortest path on positive-weighted directed graph.
pub fn dijkstra_shortest_path(
    edges: &[(usize, usize, f64)],
    n: usize,
    start: usize,
    goal: usize,
) -> Option<(f64, Vec<usize>)> {
    if start >= n || goal >= n {
        return None;
    }

    let mut adj = vec![vec![]; n];
    for &(u, v, w) in edges {
        if u < n && v < n && w >= 0.0 {
            adj[u].push((v, w));
        }
    }

    let mut dist = vec![f64::INFINITY; n];
    let mut prev = vec![None; n];
    let mut heap = BinaryHeap::new();

    dist[start] = 0.0;
    heap.push(State { cost: 0.0, position: start });

    while let Some(State { cost, position }) = heap.pop() {
        if position == goal {
            let mut path = Vec::new();
            let mut curr = goal;
            path.push(curr);
            while let Some(p) = prev[curr] {
                path.push(p);
                curr = p;
                if curr == start {
                    break;
                }
            }
            path.reverse();
            return Some((cost, path));
        }

        if cost > dist[position] {
            continue;
        }

        for &(next_node, weight) in &adj[position] {
            let next_cost = cost + weight;
            if next_cost < dist[next_node] {
                dist[next_node] = next_cost;
                prev[next_node] = Some(position);
                heap.push(State { cost: next_cost, position: next_node });
            }
        }
    }

    None
}

/// Topological Sort using Kahn's algorithm (returns None if cycle exists).
pub fn topological_sort(n: usize, directed_edges: &[(usize, usize)]) -> Option<Vec<usize>> {
    let mut in_degree = vec![0usize; n];
    let mut adj = vec![vec![]; n];

    for &(u, v) in directed_edges {
        if u < n && v < n {
            adj[u].push(v);
            in_degree[v] += 1;
        }
    }

    let mut queue = VecDeque::new();
    for (i, &deg) in in_degree.iter().enumerate().take(n) {
        if deg == 0 {
            queue.push_back(i);
        }
    }

    let mut order = Vec::with_capacity(n);
    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &v in &adj[u] {
            in_degree[v] -= 1;
            if in_degree[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    if order.len() == n {
        Some(order)
    } else {
        None // Cycle detected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dijkstra_path() {
        let edges = vec![
            (0, 1, 1.0),
            (1, 2, 2.0),
            (0, 2, 5.0),
        ];
        let (cost, path) = dijkstra_shortest_path(&edges, 3, 0, 2).unwrap();
        assert_eq!(cost, 3.0); // 0 -> 1 -> 2 is cheaper than direct 0 -> 2
        assert_eq!(path, vec![0, 1, 2]);
    }

    #[test]
    fn test_topological_sort() {
        let edges = vec![(0, 1), (1, 2), (0, 2)];
        let order = topological_sort(3, &edges).unwrap();
        assert_eq!(order, vec![0, 1, 2]);

        // Cycle test
        let cycle = vec![(0, 1), (1, 2), (2, 0)];
        assert!(topological_sort(3, &cycle).is_none());
    }

    #[test]
    fn test_page_rank() {
        let adj = vec![
            0.0, 1.0, 1.0,
            0.0, 0.0, 1.0,
            1.0, 0.0, 0.0,
        ];
        let pr = page_rank(&adj, 3, 0.85, 20);
        assert_eq!(pr.len(), 3);
        let sum: f64 = pr.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4);
    }
}
