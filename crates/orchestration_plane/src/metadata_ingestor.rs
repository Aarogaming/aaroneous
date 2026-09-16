use serde::{Deserialize, Serialize};
#[derive(Debug, Clone)]
pub struct MetadataIngestorConfig;
impl Default for MetadataIngestorConfig {
    fn default() -> Self {
        Self
    }
}
pub struct MetadataIngestor;
impl MetadataIngestor {
    pub async fn ingest(&self, _source: &str) -> Result<Vec<MetadataEvent>, anyhow::Error> {
        Ok(Vec::new())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetadataEvent {
    AnalysisComplete(MetadataAnalysis),
    Error(String),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataAnalysis;
