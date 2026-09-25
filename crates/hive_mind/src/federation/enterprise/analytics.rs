/// Analytics and Reporting Engine
///
/// Collect metrics, analyze trends, and generate reports
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub event_id: String,
    pub event_type: String,
    pub timestamp_ms: u64,
    pub value: f32,
    pub tags: HashMap<String, String>,
}

impl AnalyticsEvent {
    pub fn new(event_type: String, value: f32) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            value,
            tags: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analytics {
    pub events: Vec<AnalyticsEvent>,
    pub metrics: HashMap<String, MetricSummary>,
    pub max_events: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSummary {
    pub name: String,
    pub count: u64,
    pub sum: f32,
    pub average: f32,
    pub min: f32,
    pub max: f32,
}

impl Analytics {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            metrics: HashMap::new(),
            max_events: 10000,
        }
    }

    pub fn record_event(&mut self, event: AnalyticsEvent) {
        // Maintain size limit
        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }

        // Update metrics
        let metric = self
            .metrics
            .entry(event.event_type.clone())
            .or_insert_with(|| MetricSummary {
                name: event.event_type.clone(),
                count: 0,
                sum: 0.0,
                average: 0.0,
                min: f32::MAX,
                max: f32::MIN,
            });

        metric.count += 1;
        metric.sum += event.value;
        metric.average = metric.sum / metric.count as f32;
        metric.min = metric.min.min(event.value);
        metric.max = metric.max.max(event.value);

        self.events.push(event);
    }

    pub fn get_metric(&self, name: &str) -> Option<MetricSummary> {
        self.metrics.get(name).cloned()
    }

    pub fn generate_report(&self, report_type: &str) -> Result<String, String> {
        match report_type {
            "summary" => {
                let mut report = String::from("ANALYTICS SUMMARY\n");
                report.push_str(&format!("Total events: {}\n", self.events.len()));
                report.push_str(&format!("Tracked metrics: {}\n", self.metrics.len()));

                for (name, metric) in &self.metrics {
                    report.push_str(&format!(
                        "  {}: count={}, avg={:.2}, min={:.2}, max={:.2}\n",
                        name, metric.count, metric.average, metric.min, metric.max
                    ));
                }
                Ok(report)
            }
            _ => Err(format!("Unknown report type: {}", report_type)),
        }
    }
}

impl Default for Analytics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub report_id: String,
    pub report_type: String,
    pub generated_at_ms: u64,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_event() {
        let event = AnalyticsEvent::new("proposal_created".to_string(), 1.0);
        assert_eq!(event.event_type, "proposal_created");
    }

    #[test]
    fn test_analytics_record() {
        let mut analytics = Analytics::new();
        let event = AnalyticsEvent::new("proposal_created".to_string(), 1.0);
        analytics.record_event(event);
        assert_eq!(analytics.events.len(), 1);
    }

    #[test]
    fn test_analytics_metrics() {
        let mut analytics = Analytics::new();
        analytics.record_event(AnalyticsEvent::new("test".to_string(), 10.0));
        analytics.record_event(AnalyticsEvent::new("test".to_string(), 20.0));

        let metric = analytics.get_metric("test").unwrap();
        assert_eq!(metric.count, 2);
        assert_eq!(metric.average, 15.0);
    }
}
