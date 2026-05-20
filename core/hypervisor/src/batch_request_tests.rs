#[cfg(test)]
mod batch_request_tests {
    use crate::llm::{LLMBatchRequestManager, BatchRequestConfig};
    use crate::llm::LLMClient;
    use std::sync::Arc;

    #[test]
    fn test_batch_config_creation() {
        let config = BatchRequestConfig {
            batch_size: 10,
            batch_timeout_ms: 2000,
            max_queue_size: 200,
            enabled: true,
        };

        assert_eq!(config.batch_size, 10);
        assert_eq!(config.batch_timeout_ms, 2000);
        assert_eq!(config.max_queue_size, 200);
        assert!(config.enabled);
    }

    #[test]
    fn test_batch_config_default() {
        let config = BatchRequestConfig::default();
        assert_eq!(config.batch_size, 5);
        assert_eq!(config.batch_timeout_ms, 1000);
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_batch_manager_creation() {
        let config = BatchRequestConfig::default();
        let llm_client = Arc::new(LLMClient::new(Default::default()));
        let manager = LLMBatchRequestManager::new(config, llm_client);

        let queue_size = manager.get_queue_size().await;
        assert_eq!(queue_size, 0);
    }

    #[tokio::test]
    async fn test_batch_manager_stats_initial() {
        let config = BatchRequestConfig::default();
        let llm_client = Arc::new(LLMClient::new(Default::default()));
        let manager = LLMBatchRequestManager::new(config, llm_client);

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_batches, 0);
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.avg_batch_size, 0.0);
    }

    #[tokio::test]
    async fn test_batch_manager_stats_reset() {
        let config = BatchRequestConfig::default();
        let llm_client = Arc::new(LLMClient::new(Default::default()));
        let manager = LLMBatchRequestManager::new(config, llm_client);

        manager.reset_stats().await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_batches, 0);
    }

    #[test]
    fn test_batch_stats_calculation() {
        let mut stats = crate::llm::BatchStats::default();
        stats.total_batches = 10;
        stats.total_requests = 50;
        stats.avg_batch_size = stats.total_requests as f64 / stats.total_batches as f64;

        assert_eq!(stats.avg_batch_size, 5.0);
    }

    #[test]
    fn test_batch_stats_clone() {
        let stats = crate::llm::BatchStats {
            total_batches: 5,
            total_requests: 25,
            avg_batch_size: 5.0,
            total_processing_time_ms: 1000,
            batches_by_timeout: 2,
            batches_by_size: 3,
        };

        let cloned = stats.clone();
        assert_eq!(cloned.total_batches, stats.total_batches);
        assert_eq!(cloned.total_requests, stats.total_requests);
    }

    #[test]
    fn test_batch_config_can_disable() {
        let config = BatchRequestConfig {
            enabled: false,
            ..Default::default()
        };

        assert!(!config.enabled);
    }

    #[test]
    fn test_batch_size_adjustment() {
        let mut config = BatchRequestConfig::default();
        config.batch_size = 20;
        assert_eq!(config.batch_size, 20);
    }

    #[test]
    fn test_batch_timeout_adjustment() {
        let mut config = BatchRequestConfig::default();
        config.batch_timeout_ms = 5000;
        assert_eq!(config.batch_timeout_ms, 5000);
    }

    #[test]
    fn test_queue_size_limit() {
        let config = BatchRequestConfig {
            max_queue_size: 50,
            ..Default::default()
        };

        assert_eq!(config.max_queue_size, 50);
    }

    #[tokio::test]
    async fn test_batch_manager_multiple_operations() {
        let config = BatchRequestConfig {
            batch_size: 10,
            ..Default::default()
        };
        let llm_client = Arc::new(LLMClient::new(Default::default()));
        let manager = LLMBatchRequestManager::new(config, llm_client);

        // Verify initial state
        let initial_size = manager.get_queue_size().await;
        assert_eq!(initial_size, 0);

        // Reset stats
        manager.reset_stats().await;
        let stats_after_reset = manager.get_stats().await;
        assert_eq!(stats_after_reset.total_batches, 0);
    }

    #[test]
    fn test_batch_request_config_builder() {
        let config = BatchRequestConfig {
            batch_size: 5,
            batch_timeout_ms: 1000,
            max_queue_size: 100,
            enabled: true,
        };

        // Verify all fields are set correctly
        assert_eq!(config.batch_size, 5);
        assert_eq!(config.batch_timeout_ms, 1000);
        assert_eq!(config.max_queue_size, 100);
        assert!(config.enabled);
    }

    #[test]
    fn test_batch_size_edge_cases() {
        let config_small = BatchRequestConfig {
            batch_size: 1,
            ..Default::default()
        };

        let config_large = BatchRequestConfig {
            batch_size: 1000,
            ..Default::default()
        };

        assert_eq!(config_small.batch_size, 1);
        assert_eq!(config_large.batch_size, 1000);
    }

    #[tokio::test]
    async fn test_batch_manager_performance_metrics() {
        let config = BatchRequestConfig::default();
        let llm_client = Arc::new(LLMClient::new(Default::default()));
        let manager = LLMBatchRequestManager::new(config, llm_client);

        let stats = manager.get_stats().await;
        
        // Verify stats structure is complete
        assert_eq!(stats.total_batches, 0);
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.avg_batch_size, 0.0);
        assert_eq!(stats.total_processing_time_ms, 0);
        assert_eq!(stats.batches_by_timeout, 0);
        assert_eq!(stats.batches_by_size, 0);
    }

    #[test]
    fn test_concurrent_batch_configuration() {
        let configs = vec![
            BatchRequestConfig {
                batch_size: 3,
                batch_timeout_ms: 500,
                ..Default::default()
            },
            BatchRequestConfig {
                batch_size: 10,
                batch_timeout_ms: 2000,
                ..Default::default()
            },
            BatchRequestConfig {
                batch_size: 5,
                batch_timeout_ms: 1000,
                ..Default::default()
            },
        ];

        assert_eq!(configs.len(), 3);
        assert_eq!(configs[0].batch_size, 3);
        assert_eq!(configs[1].batch_size, 10);
        assert_eq!(configs[2].batch_size, 5);
    }

    #[test]
    fn test_batch_stats_atomic_operations() {
        let mut stats = crate::llm::BatchStats::default();
        
        // Simulate batch processing
        stats.total_batches += 1;
        stats.total_requests += 5;
        stats.total_processing_time_ms += 100;
        stats.batches_by_size += 1;

        assert_eq!(stats.total_batches, 1);
        assert_eq!(stats.total_requests, 5);
        assert_eq!(stats.total_processing_time_ms, 100);
        assert_eq!(stats.batches_by_size, 1);
    }

    #[tokio::test]
    async fn test_batch_manager_config_persistence() {
        let original_config = BatchRequestConfig {
            batch_size: 7,
            batch_timeout_ms: 3000,
            max_queue_size: 150,
            enabled: true,
        };

        let llm_client = Arc::new(LLMClient::new(Default::default()));
        let manager = LLMBatchRequestManager::new(original_config.clone(), llm_client);

        // Configuration should be preserved
        assert_eq!(manager.config.batch_size, 7);
        assert_eq!(manager.config.batch_timeout_ms, 3000);
        assert_eq!(manager.config.max_queue_size, 150);
    }

    #[test]
    fn test_batch_throughput_calculation() {
        let mut stats = crate::llm::BatchStats::default();
        stats.total_batches = 100;
        stats.total_requests = 500;
        stats.total_processing_time_ms = 10000; // 10 seconds

        let throughput = stats.total_requests as f64 / (stats.total_processing_time_ms as f64 / 1000.0);
        assert!(throughput > 0.0);
        assert_eq!(throughput, 50.0); // 50 requests per second
    }
}
