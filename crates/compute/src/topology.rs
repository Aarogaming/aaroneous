//! crates/compute/src/topology.rs
//! Topological Data Analysis (TDA) and Structural Graph Topology primitives.
//! Used for structural codebase analysis, module cluster persistence, architecture health verification, and manifold indexing.

/// Iterative Disjoint Set Union (Union-Find) with path compression and union by rank.
#[derive(Debug, Clone)]
pub struct DisjointSetUnion {
    parent: Vec<usize>,
    rank: Vec<usize>,
    count: usize,
}

impl DisjointSetUnion {
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
            count: n,
        }
    }

    /// Iterative find with two-pass full path compression (guaranteed zero recursion stack overflow).
    pub fn find(&mut self, mut x: usize) -> usize {
        if x >= self.parent.len() {
            return x;
        }
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        // Path compression
        while self.parent[x] != root {
            let next = self.parent[x];
            self.parent[x] = root;
            x = next;
        }
        root
    }

    /// Unions two elements by rank; returns true if they were previously in different components.
    pub fn union(&mut self, x: usize, y: usize) -> bool {
        if x >= self.parent.len() || y >= self.parent.len() {
            return false;
        }
        let root_x = self.find(x);
        let root_y = self.find(y);
        if root_x == root_y {
            return false;
        }

        match self.rank[root_x].cmp(&self.rank[root_y]) {
            std::cmp::Ordering::Less => self.parent[root_x] = root_y,
            std::cmp::Ordering::Greater => self.parent[root_y] = root_x,
            std::cmp::Ordering::Equal => {
                self.parent[root_y] = root_x;
                self.rank[root_x] += 1;
            }
        }
        self.count -= 1;
        true
    }

    /// Returns the number of disjoint connected components.
    pub fn component_count(&self) -> usize {
        self.count
    }

    /// Checks if x and y belong to the same component.
    pub fn connected(&mut self, x: usize, y: usize) -> bool {
        self.find(x) == self.find(y)
    }
}

/// Compute persistent homology 0-dim (connected components) via iterative DSU (backwards-compatible).
pub fn connected_components(edges: &[(usize, usize)], n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let mut dsu = DisjointSetUnion::new(n);
    for &(u, v) in edges {
        dsu.union(u, v);
    }
    dsu.component_count()
}

/// Betti number approximation (simplified backwards-compatible with 0.5 threshold).
pub fn betti_0_approx(adj: &[f64], n: usize) -> usize {
    betti_numbers(adj, n, 0.5).0
}

/// Computes Betti numbers (b0 = connected components, b1 = 1-dimensional independent cycles) for a 1-skeleton graph.
pub fn betti_numbers(adj: &[f64], n: usize, threshold: f64) -> (usize, usize) {
    if n == 0 || adj.len() < n * n {
        return (0, 0);
    }

    let mut dsu = DisjointSetUnion::new(n);
    let mut unique_edges = 0;

    for i in 0..n {
        for j in (i + 1)..n {
            if adj[i * n + j] > threshold {
                unique_edges += 1;
                dsu.union(i, j);
            }
        }
    }

    let b0 = dsu.component_count();
    // In a 1-dimensional simplicial complex (graph): b1 = E - V + b0
    let b1 = (unique_edges + b0).saturating_sub(n);

    (b0, b1)
}

/// Structural Topology Report for codebase or architecture dependency graphs.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphTopologyReport {
    pub vertices: usize,
    pub edges: usize,
    pub betti_0: usize,
    pub betti_1: usize,
    pub euler_characteristic: i64,
    pub has_cycles: bool,
    pub graph_density: f64,
}

/// Analyzes comprehensive topological properties of an undirected adjacency matrix.
pub fn analyze_graph_topology(adj: &[f64], n: usize, threshold: f64) -> GraphTopologyReport {
    let (b0, b1) = betti_numbers(adj, n, threshold);
    let mut edges = 0;
    for i in 0..n {
        for j in (i + 1)..n {
            if adj[i * n + j] > threshold {
                edges += 1;
            }
        }
    }

    let max_edges = if n > 1 { (n * (n - 1)) / 2 } else { 1 };
    let density = edges as f64 / max_edges as f64;
    let euler = (n as i64) - (edges as i64);

    GraphTopologyReport {
        vertices: n,
        edges,
        betti_0: b0,
        betti_1: b1,
        euler_characteristic: euler,
        has_cycles: b1 > 0,
        graph_density: density,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disjoint_set_union() {
        let mut dsu = DisjointSetUnion::new(5);
        assert_eq!(dsu.component_count(), 5);

        assert!(dsu.union(0, 1));
        assert!(dsu.union(1, 2));
        assert!(!dsu.union(0, 2)); // already connected
        assert_eq!(dsu.component_count(), 3);

        assert!(dsu.connected(0, 2));
        assert!(!dsu.connected(0, 3));
    }

    #[test]
    fn test_betti_cycle_detection() {
        // Triangle graph: 3 vertices, 3 edges -> 1 component (b0=1), 1 cycle (b1=1)
        let adj = vec![
            0.0, 1.0, 1.0,
            1.0, 0.0, 1.0,
            1.0, 1.0, 0.0,
        ];
        let (b0, b1) = betti_numbers(&adj, 3, 0.5);
        assert_eq!(b0, 1);
        assert_eq!(b1, 1);

        let report = analyze_graph_topology(&adj, 3, 0.5);
        assert!(report.has_cycles);
        assert_eq!(report.euler_characteristic, 0); // V - E = 3 - 3 = 0
    }

    #[test]
    fn test_tree_graph_no_cycles() {
        // Line graph: 0-1-2 -> 3 vertices, 2 edges -> b0=1, b1=0
        let adj = vec![
            0.0, 1.0, 0.0,
            1.0, 0.0, 1.0,
            0.0, 1.0, 0.0,
        ];
        let (b0, b1) = betti_numbers(&adj, 3, 0.5);
        assert_eq!(b0, 1);
        assert_eq!(b1, 0);

        let report = analyze_graph_topology(&adj, 3, 0.5);
        assert!(!report.has_cycles);
        assert_eq!(report.euler_characteristic, 1); // V - E = 3 - 2 = 1
    }
}
