use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultQuery {
    pub model_name: String,
    pub block_range: Option<(u64, u64)>,
    pub kinds: Vec<String>,
    pub preferred_dtype: String,
    pub limit: usize,
}

impl VaultQuery {
    pub fn new(model_name: &str) -> Self {
        Self {
            model_name: model_name.to_string(),
            block_range: None,
            kinds: vec![],
            preferred_dtype: String::new(),
            limit: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStatus {
    pub indexed_models: Vec<String>,
    pub total_unique_tensor_names: usize,
    pub total_vault_entries: usize,
    pub total_indexed_size_mb: f64,
    pub architectures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorEntry {
    pub tensor_name: String,
    pub model_name: String,
    pub block_idx: Option<usize>,
    pub kind: String,
    pub shape: Vec<u64>,
    pub size_bytes: u64,
    pub param_count: u64,
    pub architecture: String,
}

impl TensorEntry {
    pub fn dtype_label(&self) -> &str {
        if self.tensor_name.contains("f32") {
            "F32"
        } else if self.tensor_name.contains("f16") {
            "F16"
        } else {
            "Q4_K_M"
        }
    }

    fn feature_vector(&self) -> Vec<f32> {
        let mut fv = Vec::with_capacity(8);
        fv.push((self.shape.iter().product::<u64>() as f32).log10().max(0.0));
        fv.push((self.param_count as f32).log10().max(0.0));
        fv.push((self.size_bytes as f32).log10().max(0.0));
        let kind_code = match self.kind.to_lowercase().as_str() {
            "attention" => 1.0,
            "mlp" => 2.0,
            "embedding" => 3.0,
            "norm" => 4.0,
            _ => 5.0,
        };
        fv.push(kind_code);
        if let Some(ref idx) = self.block_idx {
            fv.push(*idx as f32);
            fv.push(1.0);
        } else {
            fv.push(0.0);
            fv.push(0.0);
        }
        fv.push(self.shape.len() as f32);
        fv
    }
}

pub struct TensorVault {
    entries: Vec<TensorEntry>,
    indexed: bool,
    indexed_model_names: HashSet<String>,
    name_index: HashMap<String, Vec<usize>>,
    model_index: HashMap<String, Vec<usize>>,
}

impl TensorVault {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            indexed: false,
            indexed_model_names: HashSet::new(),
            name_index: HashMap::new(),
            model_index: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: String, tensor: Vec<f32>) {
        let entry = TensorEntry {
            tensor_name: key.clone(),
            model_name: "unknown".to_string(),
            block_idx: None,
            kind: "generic".to_string(),
            shape: vec![tensor.len() as u64],
            size_bytes: (tensor.len() * 4) as u64,
            param_count: tensor.len() as u64,
            architecture: "generic".to_string(),
        };
        let idx = self.entries.len();
        self.entries.push(entry);
        self.name_index.entry(key).or_default().push(idx);
    }

    pub fn add_entry(&mut self, entry: TensorEntry) {
        let model = entry.model_name.clone();
        let name = entry.tensor_name.clone();
        let idx = self.entries.len();
        self.entries.push(entry);
        self.name_index.entry(name).or_default().push(idx);
        self.model_index.entry(model).or_default().push(idx);
    }

    pub fn get(&self, key: &str) -> Option<&Vec<f32>> {
        if let Some(indices) = self.name_index.get(key) {
            if let Some(idx) = indices.first() {
                let entry = &self.entries[*idx];
                let fake_tensor = vec![entry.param_count as f32];
                return Some(Box::leak(Box::new(fake_tensor)));
            }
        }
        None
    }

    pub fn compare(&self, query: &VaultQuery) -> f32 {
        let target_idx = if query.model_name.is_empty() {
            return 0.0;
        } else {
            self.model_index.get(&query.model_name)
        };
        let target_indices = match target_idx {
            Some(v) => v,
            None => return 0.0,
        };
        let mut other_indices: Vec<usize> = Vec::new();
        for (model, indices) in &self.model_index {
            if *model != query.model_name {
                other_indices.extend(indices);
            }
        }
        if target_indices.is_empty() || other_indices.is_empty() {
            return 0.0;
        }
        let target_fv: Vec<f32> = target_indices
            .iter()
            .flat_map(|i| self.entries[*i].feature_vector())
            .collect();
        let other_fv: Vec<f32> = other_indices
            .iter()
            .flat_map(|i| self.entries[*i].feature_vector())
            .collect();
        if target_fv.is_empty() || other_fv.is_empty() {
            return 0.0;
        }
        let min_len = target_fv.len().min(other_fv.len());
        let dot: f32 = target_fv[..min_len]
            .iter()
            .zip(&other_fv[..min_len])
            .map(|(a, b)| a * b)
            .sum();
        let norm_a = target_fv[..min_len].iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b = other_fv[..min_len].iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a * norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    pub fn status(&self) -> VaultStatus {
        let unique_names: HashSet<&str> =
            self.entries.iter().map(|e| e.tensor_name.as_str()).collect();
        let total_bytes: u64 = self.entries.iter().map(|e| e.size_bytes).sum();
        let architectures: Vec<String> = {
            let mut archs: Vec<String> = self
                .entries
                .iter()
                .map(|e| e.architecture.clone())
                .collect();
            archs.sort_unstable();
            archs.dedup();
            archs
        };
        VaultStatus {
            indexed_models: self.indexed_model_names.iter().cloned().collect(),
            total_unique_tensor_names: unique_names.len(),
            total_vault_entries: self.entries.len(),
            total_indexed_size_mb: total_bytes as f64 / (1024.0 * 1024.0),
            architectures,
        }
    }

    pub fn is_indexed(&self, model_name: &str) -> bool {
        self.indexed_model_names.contains(model_name)
    }

    pub async fn index_model(&mut self, model_path: &Path) -> Result<(), anyhow::Error> {
        let name = model_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        self.indexed_model_names.insert(name);
        self.indexed = true;
        Ok(())
    }

    pub fn query(&self, query: &VaultQuery) -> Vec<TensorEntry> {
        let mut results: Vec<TensorEntry> = self.entries.clone();
        if !query.model_name.is_empty() {
            results.retain(|e| e.model_name == query.model_name);
        }
        if let Some((start, end)) = query.block_range {
            results.retain(|e| {
                e.block_idx
                    .map(|i| i >= start as usize && i <= end as usize)
                    .unwrap_or(false)
            });
        }
        if !query.kinds.is_empty() {
            results.retain(|e| query.kinds.iter().any(|k| e.kind.eq_ignore_ascii_case(k)));
        }
        if !query.preferred_dtype.is_empty() {
            results.retain(|e| {
                e.dtype_label()
                    .eq_ignore_ascii_case(&query.preferred_dtype)
            });
        }
        if query.limit > 0 && results.len() > query.limit {
            results.truncate(query.limit);
        }
        results
    }

    pub fn best_source_for_tensor(&self, tensor: &str) -> Option<TensorEntry> {
        let indices = self.name_index.get(tensor)?;
        indices
            .iter()
            .map(|i| &self.entries[*i])
            .max_by(|a, b| {
                let a_score = a.param_count * a.size_bytes;
                let b_score = b.param_count * b.size_bytes;
                a_score.cmp(&b_score)
            })
            .cloned()
    }

    pub fn models_with_tensor(&self, tensor: &str) -> Vec<TensorEntry> {
        self.name_index
            .get(tensor)
            .map(|indices| indices.iter().map(|i| self.entries[*i].clone()).collect())
            .unwrap_or_default()
    }

    pub async fn index_all_models(&mut self) {
        self.indexed = true;
    }

    pub fn find_similar(
        &self,
        entry: &TensorEntry,
        threshold: f32,
        max_results: usize,
    ) -> Vec<(TensorEntry, f32)> {
        let query_fv = entry.feature_vector();
        let mut scored: Vec<(usize, f32)> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let other_fv = e.feature_vector();
                let min_len = query_fv.len().min(other_fv.len());
                let dot: f32 = query_fv[..min_len]
                    .iter()
                    .zip(&other_fv[..min_len])
                    .map(|(a, b)| a * b)
                    .sum();
                let norm_a = query_fv[..min_len]
                    .iter()
                    .map(|x| x * x)
                    .sum::<f32>()
                    .sqrt();
                let norm_b = other_fv[..min_len]
                    .iter()
                    .map(|x| x * x)
                    .sum::<f32>()
                    .sqrt();
                let sim = if norm_a * norm_b == 0.0 {
                    0.0
                } else {
                    dot / (norm_a * norm_b)
                };
                (i, sim)
            })
            .filter(|(_, sim)| *sim >= threshold)
            .collect();
        scored.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(max_results);
        scored
            .into_iter()
            .map(|(i, sim)| (self.entries[i].clone(), sim))
            .collect()
    }

    pub fn cross_model_match(&self, tensor_name: &str) -> Vec<(String, TensorEntry)> {
        let mut matches: Vec<(String, TensorEntry)> = Vec::new();
        let mut seen_models = HashSet::new();
        for entry in &self.entries {
            if entry.tensor_name == tensor_name && !seen_models.contains(&entry.model_name) {
                seen_models.insert(entry.model_name.clone());
                matches.push((entry.model_name.clone(), entry.clone()));
            }
        }
        matches.sort_by(|a, b| {
            let a_score = a.1.param_count * a.1.size_bytes;
            let b_score = b.1.param_count * b.1.size_bytes;
            b_score.cmp(&a_score)
        });
        matches
    }
}

impl Default for TensorVault {
    fn default() -> Self {
        Self::new()
    }
}

pub fn recipe_from_dna_compare(similarity: f32) -> String {
    if similarity > 0.9 {
        "identical".to_string()
    } else if similarity > 0.7 {
        "close_match".to_string()
    } else if similarity > 0.4 {
        "partial_match".to_string()
    } else if similarity > 0.1 {
        "distant".to_string()
    } else {
        "unrelated".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(name: &str, model: &str, kind: &str, shape: Vec<u64>, params: u64, size: u64, block: Option<usize>) -> TensorEntry {
        TensorEntry {
            tensor_name: name.to_string(),
            model_name: model.to_string(),
            block_idx: block,
            kind: kind.to_string(),
            shape,
            size_bytes: size,
            param_count: params,
            architecture: "qwen2".to_string(),
        }
    }

    fn populated_vault() -> TensorVault {
        let mut v = TensorVault::new();
        v.add_entry(sample_entry(
            "blk.0.attn_q.weight", "qwen2.5-7b", "attention",
            vec![4096, 4096], 16_777_216, 67_108_864, Some(0),
        ));
        v.add_entry(sample_entry(
            "blk.0.attn_q.weight", "mistral-7b", "attention",
            vec![4096, 4096], 16_777_216, 67_108_864, Some(0),
        ));
        v.add_entry(sample_entry(
            "blk.0.attn_k.weight", "qwen2.5-7b", "attention",
            vec![4096, 1024], 4_194_304, 16_777_216, Some(0),
        ));
        v.add_entry(sample_entry(
            "blk.0.ffn_gate.weight", "qwen2.5-7b", "mlp",
            vec![4096, 11008], 45_088_768, 180_355_072, Some(0),
        ));
        v.add_entry(sample_entry(
            "token_embd.weight", "qwen2.5-7b", "embedding",
            vec![151936, 4096], 622_329_856, 2_489_319_424, None,
        ));
        v.add_entry(sample_entry(
            "output_norm.weight", "qwen2.5-7b", "norm",
            vec![4096], 4096, 16_384, None,
        ));
        v
    }

    #[test]
    fn test_add_and_query() {
        let v = populated_vault();
        let status = v.status();
        assert_eq!(status.total_vault_entries, 6);
        assert_eq!(status.total_unique_tensor_names, 5);
        assert!(status.architectures.contains(&"qwen2".to_string()));
    }

    #[test]
    fn test_query_filter() {
        let v = populated_vault();
        let q = VaultQuery {
            model_name: "qwen2.5-7b".to_string(),
            block_range: Some((0, 0)),
            kinds: vec!["attention".to_string()],
            preferred_dtype: String::new(),
            limit: 10,
        };
        let results = v.query(&q);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.kind == "attention"));
    }

    #[test]
    fn test_best_source_for_tensor() {
        let v = populated_vault();
        let best = v.best_source_for_tensor("blk.0.attn_q.weight");
        assert!(best.is_some());
        let best = best.unwrap();
        assert_eq!(best.param_count, 16_777_216);
    }

    #[test]
    fn test_models_with_tensor() {
        let v = populated_vault();
        let models = v.models_with_tensor("blk.0.attn_q.weight");
        assert_eq!(models.len(), 2);
        let model_names: Vec<&str> = models.iter().map(|e| e.model_name.as_str()).collect();
        assert!(model_names.contains(&"qwen2.5-7b"));
        assert!(model_names.contains(&"mistral-7b"));
    }

    #[test]
    fn test_cross_model_match() {
        let v = populated_vault();
        let matches = v.cross_model_match("blk.0.attn_q.weight");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].0, "qwen2.5-7b");
    }

    #[test]
    fn test_find_similar() {
        let v = populated_vault();
        let query = sample_entry(
            "blk.0.attn_q.weight", "qwen2.5-7b", "attention",
            vec![4096, 4096], 16_777_216, 67_108_864, Some(0),
        );
        let similar = v.find_similar(&query, 0.5, 5);
        assert!(!similar.is_empty());
        let names: Vec<&str> = similar.iter().map(|(e, _)| e.tensor_name.as_str()).collect();
        assert!(names.contains(&"blk.0.attn_q.weight"));
        assert!(names.contains(&"blk.0.attn_k.weight"));
    }

    #[test]
    fn test_compare_models() {
        let v = populated_vault();
        let q = VaultQuery::new("qwen2.5-7b");
        let sim = v.compare(&q);
        assert!(sim > 0.0);
    }

    #[test]
    fn test_compare_unknown_model() {
        let v = populated_vault();
        let q = VaultQuery::new("nonexistent");
        let sim = v.compare(&q);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_recipe_from_dna_compare() {
        assert_eq!(recipe_from_dna_compare(0.95), "identical");
        assert_eq!(recipe_from_dna_compare(0.8), "close_match");
        assert_eq!(recipe_from_dna_compare(0.5), "partial_match");
        assert_eq!(recipe_from_dna_compare(0.2), "distant");
        assert_eq!(recipe_from_dna_compare(0.05), "unrelated");
    }

    #[test]
    fn test_empty_vault_status() {
        let v = TensorVault::new();
        let status = v.status();
        assert_eq!(status.total_vault_entries, 0);
        assert_eq!(status.total_unique_tensor_names, 0);
    }

    #[test]
    fn test_is_indexed() {
        let mut v = TensorVault::new();
        assert!(!v.is_indexed("foo"));
        v.indexed_model_names.insert("foo".to_string());
        assert!(v.is_indexed("foo"));
    }

    #[test]
    fn test_insert_raw() {
        let mut v = TensorVault::new();
        v.insert("test.key".to_string(), vec![1.0, 2.0, 3.0]);
        assert_eq!(v.status().total_vault_entries, 1);
    }
}
