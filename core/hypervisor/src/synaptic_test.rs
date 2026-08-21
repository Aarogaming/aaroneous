#[cfg(test)]
mod tests {
    use nervous_system::SWMRSynapse;
    use std::time::Duration;

    #[tokio::test]
    async fn test_synaptic_task_execution() {
        let synapse = SWMRSynapse::new("SAB_STORE_TEST", 1024 * 1024)
            .await
            .unwrap();

        // Simulate waiting for Python command
        println!("[Rust] Executor listening for Synaptic Task...");

        // In a real scenario, this would be an async loop.
        // For testing, we simulate the read/write.
        synapse.write_at(100, b"CALCULATING\x00").await.unwrap();

        // Simulate processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Update status to 1 (Completed)
        synapse.write_at(8, &1u32.to_le_bytes()).await.unwrap();

        println!("[Rust] Task processed. Status updated to 1.");
    }
}
