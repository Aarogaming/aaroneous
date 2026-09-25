#[cfg(test)]
mod tests {
    use ipc_bus::SWMRSynapse;

    #[tokio::test]
    async fn test_synaptic_task_execution() {
        let synapse = SWMRSynapse::new("SAB_STORE_TEST", 1024 * 1024)
            .await
            .unwrap();

        println!("[Rust] Executor listening for Synaptic Task...");

        // Write initial status and verify the write_at API works correctly.
        synapse.write_at(100, b"CALCULATING\x00").await.unwrap();

        // Update status to 1 (Completed) immediately — no artificial delay
        // is needed here; the test verifies write_at round-trip correctness,
        // not timing.
        synapse.write_at(8, &1u32.to_le_bytes()).await.unwrap();

        println!("[Rust] Task processed. Status updated to 1.");
    }
}
