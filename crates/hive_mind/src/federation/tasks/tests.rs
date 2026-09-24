/// Integration tests for the tasks module.
///
/// These tests cover the `BackgroundTaskHandle` lifecycle shared across
/// both task types. The task-specific tests live in each sub-module.

#[cfg(test)]
mod tests {
    use super::super::BackgroundTaskHandle;
    use std::sync::Arc;
    use tokio::sync::Notify;

    #[tokio::test]
    async fn test_handle_is_running_after_spawn() {
        let shutdown = Arc::new(Notify::new());
        let handle = BackgroundTaskHandle::new(
            "test-task",
            shutdown.clone(),
            tokio::spawn(async {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }),
        );
        assert!(handle.is_running().await);
    }

    #[tokio::test]
    async fn test_handle_not_running_after_shutdown() {
        let shutdown = Arc::new(Notify::new());
        let handle = BackgroundTaskHandle::new(
            "test-task",
            shutdown.clone(),
            tokio::spawn(async move {
                // Task that responds to shutdown
                shutdown.notified().await;
            }),
        );

        handle.shutdown().await;
        assert!(!handle.is_running().await);
    }

    #[tokio::test]
    async fn test_handle_shutdown_signals_task() {
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag_clone = flag.clone();
        let shutdown = Arc::new(Notify::new());

        let handle = BackgroundTaskHandle::new(
            "test-task",
            shutdown.clone(),
            tokio::spawn(async move {
                shutdown.notified().await;
                flag_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        assert!(!flag.load(std::sync::atomic::Ordering::SeqCst));
        // shutdown() waits for the task to finish (JoinHandle::await), so
        // by the time it returns the task has set the flag.
        handle.shutdown().await;
        assert!(
            flag.load(std::sync::atomic::Ordering::SeqCst),
            "flag should be set after shutdown joined the task"
        );
    }

    #[tokio::test]
    async fn test_handle_double_shutdown_is_safe() {
        let shutdown = Arc::new(Notify::new());
        let handle = BackgroundTaskHandle::new(
            "test-task",
            shutdown.clone(),
            tokio::spawn(async move {
                shutdown.notified().await;
            }),
        );

        handle.shutdown().await;
        // Second shutdown should be a safe no-op (handle already taken)
        handle.shutdown().await;
        assert!(!handle.is_running().await);
    }
}
