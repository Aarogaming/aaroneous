use crate::data_ingestion::{IngestibleData, FileFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Keywords for domain detection (semantic analysis)
const DOMAIN_KEYWORDS: &[(&str, &str, f32)] = &[
    // Database domain
    ("database", "database", 0.95),
    ("sql", "database", 0.90),
    ("query", "database", 0.85),
    ("table", "database", 0.80),
    ("schema", "database", 0.88),
    ("index", "database", 0.75),
    ("transaction", "database", 0.85),
    ("ddl", "database", 0.90),
    
    // Networking domain
    ("network", "networking", 0.95),
    ("packet", "networking", 0.90),
    ("protocol", "networking", 0.85),
    ("socket", "networking", 0.88),
    ("tcp", "networking", 0.92),
    ("udp", "networking", 0.92),
    ("ip", "networking", 0.85),
    ("latency", "networking", 0.80),
    ("bandwidth", "networking", 0.85),
    
    // Security domain
    ("security", "security", 0.95),
    ("authentication", "security", 0.92),
    ("encryption", "security", 0.90),
    ("certificate", "security", 0.88),
    ("vulnerability", "security", 0.92),
    ("firewall", "security", 0.85),
    ("threat", "security", 0.88),
    ("breach", "security", 0.90),
    
    // Performance domain
    ("performance", "performance", 0.95),
    ("latency", "performance", 0.88),
    ("throughput", "performance", 0.90),
    ("cpu", "performance", 0.85),
    ("memory", "performance", 0.85),
    ("profiling", "performance", 0.88),
    ("optimization", "performance", 0.85),
    ("benchmark", "performance", 0.90),
    
    // Development domain
    ("code", "development", 0.85),
    ("function", "development", 0.80),
    ("class", "development", 0.80),
    ("method", "development", 0.80),
    ("algorithm", "development", 0.85),
    ("debugging", "development", 0.85),
    ("testing", "development", 0.80),
    ("refactoring", "development", 0.85),
    
    // Operations/DevOps domain
    ("deployment", "operations", 0.90),
    ("container", "operations", 0.88),
    ("kubernetes", "operations", 0.95),
    ("docker", "operations", 0.95),
    ("infrastructure", "operations", 0.85),
    ("monitoring", "operations", 0.88),
    ("logging", "operations", 0.85),
    ("alerting", "operations", 0.80),
    
    // Crisis/Incident domain
    ("crash", "crisis", 0.95),
    ("failure", "crisis", 0.90),
    ("error", "crisis", 0.85),
    ("incident", "crisis", 0.95),
    ("outage", "crisis", 0.95),
    ("panic", "crisis", 0.90),
    ("fatal", "crisis", 0.90),
    ("recovery", "crisis", 0.85),
];

/// Analysis result for a piece of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAnalysis {
    /// Detected domains with confidence scores
    pub domains: HashMap<String, f32>,
    /// Extracted key information
    pub key_terms: Vec<(String, usize)>, // term and frequency
    /// Structural information (JSON schema, CSV columns, etc.)
    pub structure: StructuralAnalysis,
    /// Overall complexity score (0.0 - 1.0)
    pub complexity: f32,
}

/// Structural information about the data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralAnalysis {
    /// For JSON: detected schema fields
    pub fields: Vec<String>,
    /// For structured data: column names
    pub columns: Vec<String>,
    /// Nesting depth (for hierarchical data)
    pub nesting_depth: usize,
    /// Whether data appears to be time-series
    pub is_timeseries: bool,
    /// Estimated record count
    pub record_count: Option<usize>,
}

impl Default for StructuralAnalysis {
    fn default() -> Self {
        Self {
            fields: Vec::new(),
            columns: Vec::new(),
            nesting_depth: 0,
            is_timeseries: false,
            record_count: None,
        }
    }
}

/// Content Analyzer: Performs hybrid analysis (semantic + structural)
pub struct ContentAnalyzer;

impl ContentAnalyzer {
    /// Analyze content and extract meaningful information
    pub fn analyze(data: &IngestibleData) -> ContentAnalysis {
        let mut analysis = ContentAnalysis {
            domains: HashMap::new(),
            key_terms: Vec::new(),
            structure: StructuralAnalysis::default(),
            complexity: 0.0,
        };

        // Analyze content if available
        if let Some(content) = &data.content {
            // Semantic analysis: keyword extraction
            Self::semantic_analysis(content, &mut analysis);

            // Structural analysis based on format
            if let Some(format) = data.format {
                Self::structural_analysis(content, format, &mut analysis);
            }

            // Calculate complexity
            analysis.complexity = Self::calculate_complexity(&analysis);
        }

        analysis
    }

    /// Semantic analysis: keyword extraction and domain detection
    fn semantic_analysis(content: &str, analysis: &mut ContentAnalysis) {
        let content_lower = content.to_lowercase();
        let mut keyword_frequency: HashMap<String, usize> = HashMap::new();

        // Keyword matching
        for (keyword, domain, confidence) in DOMAIN_KEYWORDS {
            if content_lower.contains(keyword) {
                // Count occurrences
                let count = content_lower.matches(keyword).count();
                *keyword_frequency.entry(keyword.to_string()).or_insert(0) += count;

                // Track domain with confidence
                analysis
                    .domains
                    .entry(domain.to_string())
                    .and_modify(|conf| *conf = (*conf + confidence) / 2.0)
                    .or_insert(*confidence);
            }
        }

        // Sort keywords by frequency
        let mut keywords: Vec<_> = keyword_frequency.into_iter().collect();
        keywords.sort_by(|a, b| b.1.cmp(&a.1));
        analysis.key_terms = keywords.into_iter().take(20).collect();
    }

    /// Structural analysis: format-specific parsing
    fn structural_analysis(content: &str, format: FileFormat, analysis: &mut ContentAnalysis) {
        match format {
            FileFormat::Json => Self::analyze_json(content, analysis),
            FileFormat::Jsonl => Self::analyze_jsonl(content, analysis),
            FileFormat::Csv | FileFormat::Tsv => Self::analyze_csv(content, analysis, format),
            FileFormat::Log => Self::analyze_log(content, analysis),
            FileFormat::Yaml => Self::analyze_yaml(content, analysis),
            _ => {} // Other formats skipped for now
        }
    }

    /// Analyze JSON structure
    fn analyze_json(content: &str, analysis: &mut ContentAnalysis) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
            if let serde_json::Value::Object(ref obj) = value {
                analysis.structure.fields = obj.keys().cloned().collect();
                analysis.structure.nesting_depth = Self::calculate_nesting_depth(&value);
                analysis.structure.record_count = Some(1);
            }
        }
    }

    /// Analyze JSONL structure (one JSON object per line)
    fn analyze_jsonl(content: &str, analysis: &mut ContentAnalysis) {
        let lines: Vec<&str> = content.lines().collect();
        analysis.structure.record_count = Some(lines.len());

        // Sample first line to get schema
        if let Some(first_line) = lines.first() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(first_line) {
                if let serde_json::Value::Object(obj) = value {
                    analysis.structure.fields = obj.keys().cloned().collect();
                }
            }
        }

        // Check for timestamp fields (timeseries indicator)
        analysis.structure.is_timeseries =
            analysis
                .structure
                .fields
                .iter()
                .any(|f| {
                    f.to_lowercase().contains("time") || f.to_lowercase().contains("date")
                });
    }

    /// Analyze CSV/TSV structure
    fn analyze_csv(content: &str, analysis: &mut ContentAnalysis, format: FileFormat) {
        let delimiter = match format {
            FileFormat::Csv => ',',
            FileFormat::Tsv => '\t',
            _ => ',',
        };

        let lines: Vec<&str> = content.lines().collect();
        
        if !lines.is_empty() {
            // First line is typically headers
            let headers: Vec<&str> = lines[0].split(delimiter).collect();
            analysis.structure.columns = headers.iter().map(|h| h.to_string()).collect();
            analysis.structure.record_count = Some(lines.len().saturating_sub(1));

            // Check for timestamp columns
            analysis.structure.is_timeseries =
                headers
                    .iter()
                    .any(|h| {
                        h.to_lowercase().contains("time") || h.to_lowercase().contains("date")
                    });
        }
    }

    /// Analyze log file structure
    fn analyze_log(content: &str, analysis: &mut ContentAnalysis) {
        let lines: Vec<&str> = content.lines().collect();
        analysis.structure.record_count = Some(lines.len());
        analysis.structure.is_timeseries = true; // Logs are typically time-series

        // Extract common log fields
        for line in lines.iter().take(10) {
            if line.contains("[ERROR]") || line.contains("[WARN]") {
                analysis.structure.fields.push("level".to_string());
            }
            if line.contains("::") {
                analysis.structure.fields.push("module".to_string());
            }
        }

        // Remove duplicates
        analysis.structure.fields.sort();
        analysis.structure.fields.dedup();
    }

    /// Analyze YAML structure
    fn analyze_yaml(content: &str, analysis: &mut ContentAnalysis) {
        // Simple YAML field extraction (keys at top level)
        for line in content.lines().take(50) {
            if line.contains(':') && !line.trim().starts_with('#') {
                if let Some(key) = line.split(':').next() {
                    let key_clean = key.trim().to_string();
                    if !key_clean.is_empty() {
                        analysis.structure.fields.push(key_clean);
                    }
                }
            }
        }
    }

    /// Calculate nesting depth for hierarchical data
    fn calculate_nesting_depth(value: &serde_json::Value) -> usize {
        match value {
            serde_json::Value::Object(obj) => {
                let max_child_depth = obj
                    .values()
                    .map(|v| Self::calculate_nesting_depth(v))
                    .max()
                    .unwrap_or(0);
                1 + max_child_depth
            }
            serde_json::Value::Array(arr) => {
                let max_child_depth = arr
                    .iter()
                    .map(|v| Self::calculate_nesting_depth(v))
                    .max()
                    .unwrap_or(0);
                1 + max_child_depth
            }
            _ => 0,
        }
    }

    /// Calculate overall complexity score
    fn calculate_complexity(analysis: &ContentAnalysis) -> f32 {
        let mut score = 0.0;

        // Domain variety (0.3 weight)
        let domain_variety = (analysis.domains.len() as f32).min(10.0) / 10.0;
        score += domain_variety * 0.3;

        // Key term diversity (0.2 weight)
        let term_diversity = (analysis.key_terms.len() as f32).min(20.0) / 20.0;
        score += term_diversity * 0.2;

        // Structural complexity (0.3 weight)
        let nesting = (analysis.structure.nesting_depth as f32).min(5.0) / 5.0;
        let field_count = (analysis.structure.fields.len() as f32).min(50.0) / 50.0;
        score += ((nesting + field_count) / 2.0) * 0.3;

        // Timeseries detection (0.2 weight)
        if analysis.structure.is_timeseries {
            score += 0.2;
        }

        score.min(1.0)
    }

    /// Get top domains from analysis
    pub fn top_domains(analysis: &ContentAnalysis, n: usize) -> Vec<(String, f32)> {
        let mut domains: Vec<_> = analysis.domains.iter().map(|(k, v)| (k.clone(), *v)).collect();
        domains.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        domains.into_iter().take(n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_analysis_database() {
        let data = IngestibleData::from_payload(
            "SQL query on user table with transaction log".to_string(),
            "text/plain".to_string(),
        );

        let analysis = ContentAnalyzer::analyze(&data);
        let top = ContentAnalyzer::top_domains(&analysis, 3);

        assert!(!top.is_empty());
        assert_eq!(top[0].0, "database"); // Should detect database domain
    }

    #[test]
    fn test_json_structure_analysis() {
        let json_content = r#"{"name": "test", "age": 30, "nested": {"field": "value"}}"#;
        let data = IngestibleData::from_payload(
            json_content.to_string(),
            "application/json".to_string(),
        );

        let analysis = ContentAnalyzer::analyze(&data);
        assert!(!analysis.structure.fields.is_empty());
        assert!(analysis.structure.nesting_depth > 0);
    }

    #[test]
    fn test_csv_structure_analysis() {
        let csv_content = "name,age,email\nAlice,30,alice@test.com\nBob,25,bob@test.com";
        let mut data = IngestibleData::from_payload(csv_content.to_string(), "text/csv".to_string());
        data.format = Some(FileFormat::Csv);

        let analysis = ContentAnalyzer::analyze(&data);
        assert_eq!(analysis.structure.columns.len(), 3);
        assert_eq!(analysis.structure.record_count, Some(2));
    }

    #[test]
    fn test_complexity_calculation() {
        let simple = IngestibleData::from_payload("hello world".to_string(), "text/plain".to_string());
        let complex = IngestibleData::from_payload(
            "network protocol transaction database schema query optimization performance".to_string(),
            "text/plain".to_string(),
        );

        let simple_analysis = ContentAnalyzer::analyze(&simple);
        let complex_analysis = ContentAnalyzer::analyze(&complex);

        assert!(complex_analysis.complexity > simple_analysis.complexity);
    }

    #[test]
    fn test_log_analysis() {
        let log_content = "[2026-04-28 10:15:32] ERROR module::function - Connection failed\n[2026-04-28 10:15:33] WARN module::other - Retrying";
        let mut data = IngestibleData::from_payload(log_content.to_string(), "text/plain".to_string());
        data.format = Some(FileFormat::Log);

        let analysis = ContentAnalyzer::analyze(&data);
        assert!(analysis.structure.is_timeseries);
        assert_eq!(analysis.structure.record_count, Some(2));
    }

    #[test]
    fn test_crisis_domain_detection() {
        let data = IngestibleData::from_payload(
            "Database crash with fatal panic during recovery from outage".to_string(),
            "text/plain".to_string(),
        );

        let analysis = ContentAnalyzer::analyze(&data);
        let top = ContentAnalyzer::top_domains(&analysis, 5);

        assert!(top.iter().any(|(domain, _)| domain == "crisis"));
    }
}
