use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running { started_at: u64 },
    Complete { completed_at: u64, output_preview: String },
    Failed { error: String },
    Blocked { waiting_on: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeKind {
    Task { intent_id: String, assigned_to: String, status: TaskStatus },
    Model { file_path: String, model_name: String, architecture: String, tensor_count: u64, size_bytes: u64 },
    KnowledgeFragment { source_url: Option<String>, content_hash: String, confidence: f32 },
    Custom { kind_label: String, payload: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Node {
    pub fn new(id: impl Into<String>, label: impl Into<String>, kind: NodeKind) -> Self {
        Self { id: id.into(), label: label.into(), kind, tags: vec![], created_at: now_ms(), metadata: HashMap::new() }
    }
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self { self.tags.push(tag.into()); self }
    pub fn with_meta(mut self, key: impl Into<String>, val: serde_json::Value) -> Self { self.metadata.insert(key.into(), val); self }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EdgeKind {
    DependsOn,
    CrystallizedFrom { blocks_taken: Vec<usize>, tensor_count: u64 },
    DerivedFrom { method: String },
    RelatedTo { label: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
    pub weight: f32,
    pub created_at: u64,
}

impl Edge {
    pub fn new(from: impl Into<String>, to: impl Into<String>, kind: EdgeKind) -> Self {
        Self { from: from.into(), to: to.into(), kind, weight: 1.0, created_at: now_ms() }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Node '{0}' not found")] NodeNotFound(String),
    #[error("Edge would create a cycle: {0} -> {1}")] CycleDetected(String, String),
    #[error("Duplicate node id: {0}")] DuplicateNode(String),
}

#[derive(Debug, Default)]
pub struct SovereignGraph {
    nodes: HashMap<String, Node>,
    forward: HashMap<String, Vec<usize>>,
    reverse: HashMap<String, Vec<usize>>,
    edges: Vec<Edge>,
}

impl SovereignGraph {
    pub fn new() -> Self { Self::default() }

    pub fn add_node(&mut self, node: Node) -> Result<(), GraphError> {
        if self.nodes.contains_key(&node.id) { return Err(GraphError::DuplicateNode(node.id)); }
        self.forward.entry(node.id.clone()).or_default();
        self.reverse.entry(node.id.clone()).or_default();
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    pub fn add_edge(&mut self, edge: Edge) -> Result<(), GraphError> {
        if !self.nodes.contains_key(&edge.from) { return Err(GraphError::NodeNotFound(edge.from.clone())); }
        if !self.nodes.contains_key(&edge.to) { return Err(GraphError::NodeNotFound(edge.to.clone())); }
        if self.has_path(&edge.to, &edge.from) { return Err(GraphError::CycleDetected(edge.from.clone(), edge.to.clone())); }
        let idx = self.edges.len();
        self.forward.entry(edge.from.clone()).or_default().push(idx);
        self.reverse.entry(edge.to.clone()).or_default().push(idx);
        self.edges.push(edge);
        Ok(())
    }

    pub fn set_task_status(&mut self, node_id: &str, status: TaskStatus) -> Result<(), GraphError> {
        let node = self.nodes.get_mut(node_id).ok_or_else(|| GraphError::NodeNotFound(node_id.to_string()))?;
        if let NodeKind::Task { status: ref mut s, .. } = node.kind { *s = status; }
        Ok(())
    }

    pub fn node(&self, id: &str) -> Option<&Node> { self.nodes.get(id) }
    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }

    pub fn has_path(&self, from: &str, to: &str) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from.to_string());
        while let Some(current) = queue.pop_front() {
            if current == to { return true; }
            if visited.contains(&current) { continue; }
            visited.insert(current.clone());
            if let Some(idxs) = self.forward.get(&current) {
                for &i in idxs { queue.push_back(self.edges[i].to.clone()); }
            }
        }
        false
    }

    pub fn roots(&self) -> Vec<&Node> {
        self.nodes.values().filter(|n| self.reverse[&n.id].is_empty()).collect()
    }

    pub fn successors(&self, id: &str) -> Vec<&Node> {
        self.forward.get(id).map(|idxs| idxs.iter().filter_map(|&i| self.nodes.get(&self.edges[i].to)).collect()).unwrap_or_default()
    }

    pub fn predecessors(&self, id: &str) -> Vec<&Node> {
        self.reverse.get(id).map(|idxs| idxs.iter().filter_map(|&i| self.nodes.get(&self.edges[i].from)).collect()).unwrap_or_default()
    }

    pub fn topological_order(&self) -> Result<Vec<&Node>, GraphError> {
        let mut in_degree: HashMap<&str, usize> = self.nodes.keys().map(|id| (id.as_str(), self.reverse[id].len())).collect();
        let mut queue: VecDeque<&str> = in_degree.iter().filter(|(_, d)| **d == 0).map(|(id, _)| *id).collect();
        let mut result = Vec::with_capacity(self.nodes.len());
        while let Some(id) = queue.pop_front() {
            result.push(self.nodes.get(id).unwrap());
            if let Some(idxs) = self.forward.get(id) {
                for &i in idxs {
                    let succ = self.edges[i].to.as_str();
                    let deg = in_degree.get_mut(succ).unwrap();
                    *deg -= 1;
                    if *deg == 0 { queue.push_back(succ); }
                }
            }
        }
        if result.len() != self.nodes.len() { Err(GraphError::CycleDetected("?".into(), "?".into())) } else { Ok(result) }
    }

    pub fn parallel_layers(&self) -> Vec<Vec<&Node>> {
        let mut depth: HashMap<&str, usize> = HashMap::new();
        let roots: Vec<&str> = self.roots().iter().map(|n| n.id.as_str()).collect();
        let mut queue: VecDeque<(&str, usize)> = roots.iter().map(|&id| (id, 0)).collect();
        while let Some((id, d)) = queue.pop_front() {
            let current = depth.entry(id).or_insert(0);
            if d > *current { *current = d; }
            if let Some(idxs) = self.forward.get(id) {
                for &i in idxs { queue.push_back((self.edges[i].to.as_str(), d + 1)); }
            }
        }
        let max_d = depth.values().copied().max().unwrap_or(0);
        let mut layers: Vec<Vec<&Node>> = vec![vec![]; max_d + 1];
        for (id, d) in &depth {
            if let Some(node) = self.nodes.get(*id) { layers[*d].push(node); }
        }
        layers.retain(|l| !l.is_empty());
        layers
    }

    pub fn nodes_with_tag(&self, tag: &str) -> Vec<&Node> {
        self.nodes.values().filter(|n| n.tags.iter().any(|t| t == tag)).collect()
    }

    pub fn edges_from(&self, id: &str) -> Vec<&Edge> {
        self.forward.get(id).map(|idxs| idxs.iter().map(|&i| &self.edges[i]).collect()).unwrap_or_default()
    }

    pub fn bfs(&self, start: &str, max_depth: usize) -> Vec<(usize, &Node)> {
        let mut visited = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        queue.push_back((start.to_string(), 0));
        let mut result = Vec::new();
        while let Some((id, depth)) = queue.pop_front() {
            if visited.contains(&id) || depth > max_depth { continue; }
            visited.insert(id.clone());
            if let Some(node) = self.nodes.get(&id) {
                result.push((depth, node));
                if let Some(idxs) = self.forward.get(&id) {
                    for &i in idxs { queue.push_back((self.edges[i].to.clone(), depth + 1)); }
                }
            }
        }
        result
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({"node_count": self.nodes.len(), "edge_count": self.edges.len(), "nodes": self.nodes.values().collect::<Vec<_>>(), "edges": self.edges})
    }
}

pub fn task_dag_from_odin_output(intent_id: &str, json_str: &str) -> Result<SovereignGraph, Box<dyn std::error::Error>> {
    let v: serde_json::Value = serde_json::from_str(json_str)?;
    let tasks = v.get("tasks").and_then(|t| t.as_array()).ok_or("Expected {\"tasks\":[...]}")?;
    let mut graph = SovereignGraph::new();
    for (i, task) in tasks.iter().enumerate() {
        let id = task.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| format!("task-{}", i));
        let content = task.get("content").and_then(|v| v.as_str()).unwrap_or("unnamed task");
        let assign_to = task.get("assign_to").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let priority = task.get("priority").and_then(|v| v.as_str()).unwrap_or("Normal").to_string();
        let node = Node::new(id.clone(), content, NodeKind::Task { intent_id: intent_id.to_string(), assigned_to: assign_to.clone(), status: TaskStatus::Pending })
            .with_tag(&assign_to).with_tag("odin_task")
            .with_meta("priority", serde_json::Value::String(priority));
        graph.add_node(node)?;
    }
    for task in tasks {
        let id = task.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(deps) = task.get("deps").and_then(|d| d.as_array()) {
            for dep in deps {
                if let Some(dep_id) = dep.as_str() {
                    graph.add_edge(Edge::new(dep_id, id, EdgeKind::DependsOn)).ok();
                }
            }
        }
    }
    Ok(graph)
}

pub fn model_lineage_graph(registry_json: &serde_json::Value) -> SovereignGraph {
    let mut graph = SovereignGraph::new();
    if let Some(models) = registry_json.get("available_models").and_then(|m| m.as_object()) {
        for (key, meta) in models {
            let path = meta.get("path").and_then(|p| p.as_str()).unwrap_or(key);
            let name = meta.get("name").and_then(|n| n.as_str()).unwrap_or(key);
            let node = Node::new(key, name, NodeKind::Model { file_path: path.to_string(), model_name: name.to_string(), architecture: "qwen2".into(), tensor_count: 0, size_bytes: 0 }).with_tag("foundation");
            graph.add_node(node).ok();
        }
    }
    let sovereigns = [("ariel","Ariel","ariel-qwen2.5-7b.gguf"),("hermes","Hermes","hermes-qwen2.5-7b.gguf"),("wen","Wen","wen-qwen2.5-7b.gguf"),("kami","Kami","kami-qwen2.5-7b.gguf"),("dionysus","Dionysus","dionysus-qwen2.5-7b.gguf"),("merlin","Merlin","merlin-qwen2.5-7b.gguf"),("odin","Odin","odin-qwen2.5-7b.gguf"),("argus","Argus","argus-qwen2.5-7b.gguf"),("hephaestus","Hephaestus","hephaestus-qwen2.5-7b.gguf")];
    let paths = crate::workspace::WorkspacePaths::discover();
    for (id, label, filename) in &sovereigns {
        let full_path = paths.models().join(filename).to_string_lossy().to_string();
        let size = std::path::Path::new(&full_path).metadata().map(|m| m.len()).unwrap_or(0);
        let node = Node::new(*id, *label, NodeKind::Model { file_path: full_path, model_name: (*label).to_string(), architecture: "qwen2".into(), tensor_count: 0, size_bytes: size }).with_tag("sovereign").with_tag("crystallized");
        graph.add_node(node).ok();
        graph.add_edge(Edge::new("foundation_v1", *id, EdgeKind::CrystallizedFrom { blocks_taken: vec![], tensor_count: 0 })).ok();
    }
    graph
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_custom(id: &str) -> Node {
        Node::new(id, id, NodeKind::Custom { kind_label: "test".into(), payload: serde_json::Value::Null })
    }

    #[test] fn test_dag_topology() {
        let mut g = SovereignGraph::new();
        for id in &["a","b","c"] { g.add_node(make_custom(id)).unwrap(); }
        g.add_edge(Edge::new("a","b",EdgeKind::DependsOn)).unwrap();
        g.add_edge(Edge::new("b","c",EdgeKind::DependsOn)).unwrap();
        let order = g.topological_order().unwrap();
        let ids: Vec<&str> = order.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.iter().position(|&id| id=="a") < ids.iter().position(|&id| id=="b"));
    }

    #[test] fn test_cycle_detection() {
        let mut g = SovereignGraph::new();
        for id in &["x","y","z"] { g.add_node(make_custom(id)).unwrap(); }
        g.add_edge(Edge::new("x","y",EdgeKind::DependsOn)).unwrap();
        g.add_edge(Edge::new("y","z",EdgeKind::DependsOn)).unwrap();
        assert!(matches!(g.add_edge(Edge::new("z","x",EdgeKind::DependsOn)), Err(GraphError::CycleDetected(..))));
    }

    #[test] fn test_parallel_layers() {
        let mut g = SovereignGraph::new();
        for id in &["a","b","c"] { g.add_node(make_custom(id)).unwrap(); }
        g.add_edge(Edge::new("a","c",EdgeKind::DependsOn)).unwrap();
        g.add_edge(Edge::new("b","c",EdgeKind::DependsOn)).unwrap();
        let layers = g.parallel_layers();
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].len(), 2);
    }

    #[test] fn test_odin_json() {
        let json = r#"{"tasks":[{"id":"t1","content":"research","assign_to":"Merlin","deps":[]},{"id":"t2","content":"write","assign_to":"Ariel","deps":["t1"]}]}"#;
        let dag = task_dag_from_odin_output("i1", json).unwrap();
        assert_eq!(dag.node_count(), 2);
        assert!(dag.has_path("t1","t2"));
        assert!(!dag.has_path("t2","t1"));
    }
}