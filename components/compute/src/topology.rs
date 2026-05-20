/// Topological Data Analysis (TDA) primitives.
/// Used for structural codebase analysis, identifying "rot" vs "healthy" architecture.

/// Compute persistent homology 0-dim (connected components) via union-find
pub fn connected_components(edges: &[(usize, usize)], n: usize) -> usize {
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], i: usize) -> usize {
        if parent[i] != i { parent[i] = find(parent, parent[i]); }
        parent[i]
    }
    for &(u, v) in edges {
        let root_u = find(&mut parent, u);
        let root_v = find(&mut parent, v);
        if root_u != root_v { parent[root_u] = root_v; }
    }
    (0..n).filter(|&i| parent[i] == i).count()
}

/// Betti number approximation (simplified)
pub fn betti_0_approx(adj: &[f64], n: usize) -> usize {
    let edges: Vec<(usize, usize)> = adj.iter().enumerate()
        .filter(|(_, &w)| w > 0.5)
        .map(|(idx, _)| (idx / n, idx % n))
        .collect();
    connected_components(&edges, n)
}
