#[cfg(test)]
mod tests {
    use nervous_system::SharedMemorySynapse;
    use std::thread;
    use std::time::Duration;

    #[tokio::test]
    async fn test_synaptic_task_execution() {
        let synapse = SharedMemorySynapse::new("SAB_STORE", 1024 * 1024)
            .await
            .unwrap();

        // Simulate waiting for Python command
        println!("[Rust] Executor listening for Synaptic Task...");

        // Simulate processing
        synapse.write_at(100, b"CALCULATING\x00").await.unwrap();
        thread::sleep(Duration::from_millis(100));

        // Update status to 1 (Completed)
        synapse.write_at(8, &1u32.to_le_bytes()).await.unwrap();

        println!("[Rust] Task processed. Status updated to 1.");
    }
}
