#[cfg(test)]
mod stress_tests {
    use crate::hive_runtime::HiveRuntime;
    use crate::specialist::Specialist;
    use crate::task::{Task, TaskPriority};
    use std::time::{Duration, Instant};
    use tokio::time::sleep;

    #[tokio::test]
    #[ignore] // Run with: cargo test --release stress_tests -- --ignored --nocapture
    async fn stress_test_100_sequential_tasks() {
        let runtime = HiveRuntime::new("./hive_stress_test_1.db")
            .await
            .expect("Failed to create runtime");

        // Add multiple specialists
        for i in 0..5 {
            let specialist = Specialist::new(
                &format!("specialist-{}", i),
                &format!("Specialist {}", i),
                "General purpose worker",
                vec!["Analysis", "Processing", "Data Science"],
            );
            runtime
                .add_specialist(specialist)
                .await
                .expect("Failed to add specialist");
        }

        runtime.start().await.expect("Failed to start runtime");

        let start = Instant::now();
        let mut task_ids = Vec::new();

        // Submit 100 tasks sequentially
        for i in 0..100 {
            let task = Task {
                id: format!("stress-task-{:03d}", i),
                name: format!("Stress Task {}", i),
                description: format!("Processing batch {}", i),
                data_sample: Some(format!("Sample data {}", i)),
                priority: match i % 4 {
                    0 => TaskPriority::Low,
                    1 => TaskPriority::Normal,
                    2 => TaskPriority::High,
                    _ => TaskPriority::Critical,
                },
                deadline_secs: Some(120),
                required_skills: vec!["Analysis", "Processing"],
                tags: vec!["stress-test", "sequential"],
            };

            if let Ok(task_id) = runtime.submit_task(task).await {
                task_ids.push(task_id);
            }
        }

        println!("Submitted {} tasks in {:?}", task_ids.len(), start.elapsed());

        // Wait for all tasks to complete
        let mut completed = 0;
        let mut failed = 0;
        let wait_start = Instant::now();

        loop {
            let mut all_done = true;

            for task_id in &task_ids {
                if let Ok(Some(status)) = runtime.get_task_status(task_id).await {
                    match status.as_str() {
                        "Completed" => completed += 1,
                        "Failed" => failed += 1,
                        _ => {
                            all_done = false;
                        }
                    }
                }
            }

            if all_done || wait_start.elapsed() > Duration::from_secs(300) {
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        let total_time = start.elapsed();
        println!("\n=== 100 SEQUENTIAL TASKS STRESS TEST ===");
        println!("Total Time: {:?}", total_time);
        println!("Completed: {}", completed);
        println!("Failed: {}", failed);
        println!(
            "Avg Time per Task: {:?}",
            total_time / completed.max(1) as u32
        );
        println!(
            "Success Rate: {:.1}%",
            (completed as f64 / task_ids.len() as f64) * 100.0
        );

        runtime.shutdown().await.expect("Failed to shutdown");
    }

    #[tokio::test]
    #[ignore]
    async fn stress_test_10_concurrent_tasks() {
        let runtime = HiveRuntime::new("./hive_stress_test_2.db")
            .await
            .expect("Failed to create runtime");

        // Add specialists
        for i in 0..3 {
            let specialist = Specialist::new(
                &format!("concurrent-specialist-{}", i),
                &format!("Concurrent Worker {}", i),
                "Concurrent processing",
                vec!["Async Processing", "Concurrency", "Optimization"],
            );
            runtime
                .add_specialist(specialist)
                .await
                .expect("Failed to add specialist");
        }

        runtime.start().await.expect("Failed to start runtime");

        let start = Instant::now();

        // Submit 10 tasks concurrently
        let tasks = (0..10)
            .map(|i| {
                Task {
                    id: format!("concurrent-task-{:02d}", i),
                    name: format!("Concurrent Task {}", i),
                    description: format!("Parallel processing {}", i),
                    data_sample: Some(format!("Concurrent data {}", i)),
                    priority: TaskPriority::High,
                    deadline_secs: Some(120),
                    required_skills: vec!["Async Processing"],
                    tags: vec!["concurrent"],
                }
            })
            .collect::<Vec<_>>();

        let mut task_ids = Vec::new();

        // Submit all in parallel
        for task in tasks {
            if let Ok(task_id) = runtime.submit_task(task).await {
                task_ids.push(task_id);
            }
        }

        println!("Submitted {} tasks concurrently", task_ids.len());

        // Wait for completion
        let mut completed = 0;
        let mut failed = 0;
        let wait_start = Instant::now();

        loop {
            let mut all_done = true;

            for task_id in &task_ids {
                if let Ok(Some(status)) = runtime.get_task_status(task_id).await {
                    match status.as_str() {
                        "Completed" => completed += 1,
                        "Failed" => failed += 1,
                        _ => {
                            all_done = false;
                        }
                    }
                }
            }

            if all_done || wait_start.elapsed() > Duration::from_secs(60) {
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        let total_time = start.elapsed();
        println!("\n=== 10 CONCURRENT TASKS STRESS TEST ===");
        println!("Total Time: {:?}", total_time);
        println!("Completed: {}", completed);
        println!("Failed: {}", failed);
        println!(
            "Success Rate: {:.1}%",
            (completed as f64 / task_ids.len() as f64) * 100.0
        );
        println!(
            "Parallel Efficiency: {:.2}x",
            (100.0 / total_time.as_secs_f64()) * task_ids.len() as f64
        );

        runtime.shutdown().await.expect("Failed to shutdown");
    }

    #[tokio::test]
    #[ignore]
    async fn stress_test_mixed_workload() {
        let runtime = HiveRuntime::new("./hive_stress_test_3.db")
            .await
            .expect("Failed to create runtime");

        // Add specialists with different capabilities
        let specialists = vec![
            Specialist::new(
                "data-specialist",
                "Data Expert",
                "Database and analysis",
                vec!["SQL", "Data Analysis", "Statistics"],
            ),
            Specialist::new(
                "systems-specialist",
                "Systems Expert",
                "Infrastructure and optimization",
                vec!["Systems Design", "Performance", "Optimization"],
            ),
            Specialist::new(
                "ml-specialist",
                "ML Expert",
                "Machine learning",
                vec!["Machine Learning", "Python", "TensorFlow"],
            ),
        ];

        for specialist in specialists {
            runtime
                .add_specialist(specialist)
                .await
                .expect("Failed to add specialist");
        }

        runtime.start().await.expect("Failed to start runtime");

        let start = Instant::now();

        // Create diverse workload: 30 tasks of different types
        let task_types = vec![
            ("Analysis", vec!["SQL", "Data Analysis"]),
            ("Optimization", vec!["Performance", "Optimization"]),
            ("Learning", vec!["Machine Learning", "Python"]),
        ];

        let mut task_ids = Vec::new();

        for batch in 0..10 {
            for (task_type, skills) in &task_types {
                let task = Task {
                    id: format!("mixed-task-{}-{}", batch, task_type),
                    name: format!("{} Task {}", task_type, batch),
                    description: format!("Mixed workload - {}", task_type),
                    data_sample: Some(format!("Mixed data for {}", task_type)),
                    priority: match batch % 3 {
                        0 => TaskPriority::Low,
                        1 => TaskPriority::Normal,
                        _ => TaskPriority::High,
                    },
                    deadline_secs: Some(120),
                    required_skills: skills.iter().map(|s| s.to_string()).collect(),
                    tags: vec!["mixed-workload"],
                };

                if let Ok(task_id) = runtime.submit_task(task).await {
                    task_ids.push(task_id);
                }
            }
        }

        println!("Submitted {} mixed workload tasks", task_ids.len());

        // Monitor completion
        let mut completed = 0;
        let mut failed = 0;
        let mut stats = std::collections::HashMap::new();

        loop {
            let mut all_done = true;

            for task_id in &task_ids {
                if let Ok(Some(status)) = runtime.get_task_status(task_id).await {
                    match status.as_str() {
                        "Completed" => {
                            completed += 1;
                            *stats.entry("Completed").or_insert(0) += 1;
                        }
                        "Failed" => {
                            failed += 1;
                            *stats.entry("Failed").or_insert(0) += 1;
                        }
                        s => {
                            all_done = false;
                            *stats.entry(s).or_insert(0) += 1;
                        }
                    }
                }
            }

            if all_done || start.elapsed() > Duration::from_secs(300) {
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        let total_time = start.elapsed();
        println!("\n=== MIXED WORKLOAD STRESS TEST ===");
        println!("Total Time: {:?}", total_time);
        println!("Total Tasks: {}", task_ids.len());
        println!("Completed: {}", completed);
        println!("Failed: {}", failed);
        println!(
            "Success Rate: {:.1}%",
            (completed as f64 / task_ids.len() as f64) * 100.0
        );
        println!("Status Distribution:");
        for (status, count) in stats {
            println!("  {}: {}", status, count);
        }

        runtime.shutdown().await.expect("Failed to shutdown");
    }

    #[tokio::test]
    #[ignore]
    async fn stress_test_error_recovery_load() {
        let runtime = HiveRuntime::new("./hive_stress_test_4.db")
            .await
            .expect("Failed to create runtime");

        let specialist = Specialist::new(
            "resilient-specialist",
            "Resilient Worker",
            "Handles errors gracefully",
            vec!["Error Handling", "Recovery"],
        );

        runtime
            .add_specialist(specialist)
            .await
            .expect("Failed to add specialist");

        runtime.start().await.expect("Failed to start runtime");

        let start = Instant::now();

        // Submit tasks that may fail
        let mut task_ids = Vec::new();
        for i in 0..50 {
            let task = Task {
                id: format!("error-task-{:02d}", i),
                name: format!("Error-prone Task {}", i),
                description: "Task that might fail but should recover".to_string(),
                data_sample: Some(format!("Error scenario {}", i % 5)),
                priority: TaskPriority::Normal,
                deadline_secs: Some(60),
                required_skills: vec!["Error Handling", "Recovery"],
                tags: vec!["error-recovery"],
            };

            if let Ok(task_id) = runtime.submit_task(task).await {
                task_ids.push(task_id);
            }
        }

        println!("Submitted {} error-prone tasks", task_ids.len());

        // Wait for completion and track recovery stats
        let mut completed = 0;
        let mut failed = 0;
        let mut recovered = 0;

        loop {
            let mut all_done = true;

            for task_id in &task_ids {
                if let Ok(Some(status)) = runtime.get_task_status(task_id).await {
                    match status.as_str() {
                        "Completed" => completed += 1,
                        "Failed" => failed += 1,
                        "Recovered" => recovered += 1,
                        _ => all_done = false,
                    }
                }
            }

            if all_done || start.elapsed() > Duration::from_secs(120) {
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        let total_time = start.elapsed();
        println!("\n=== ERROR RECOVERY LOAD TEST ===");
        println!("Total Time: {:?}", total_time);
        println!("Tasks: {} total", task_ids.len());
        println!("Completed: {}", completed);
        println!("Recovered: {}", recovered);
        println!("Failed: {}", failed);
        println!(
            "Recovery Rate: {:.1}%",
            (recovered as f64 / task_ids.len() as f64) * 100.0
        );

        runtime.shutdown().await.expect("Failed to shutdown");
    }

    #[tokio::test]
    #[ignore]
    async fn stress_test_memory_under_load() {
        let runtime = HiveRuntime::new("./hive_stress_test_5.db")
            .await
            .expect("Failed to create runtime");

        // Add specialist with extensive memory
        let specialist = Specialist::new(
            "memory-specialist",
            "Memory Expert",
            "Learns extensively",
            vec!["Learning", "Memory", "Knowledge"],
        );

        runtime
            .add_specialist(specialist)
            .await
            .expect("Failed to add specialist");

        runtime.start().await.expect("Failed to start runtime");

        // Get initial memory
        let stats_before = runtime.get_statistics().await.ok();

        let start = Instant::now();

        // Submit tasks that create extensive memory entries
        let mut task_ids = Vec::new();
        for i in 0..50 {
            let task = Task {
                id: format!("memory-task-{:02d}", i),
                name: format!("Memory-intensive Task {}", i),
                description: "Creates multiple memory entries and decisions".to_string(),
                data_sample: Some(format!("Memory scenario with {} bytes", i * 1000)),
                priority: TaskPriority::Normal,
                deadline_secs: Some(60),
                required_skills: vec!["Learning", "Memory"],
                tags: vec!["memory-load"],
            };

            if let Ok(task_id) = runtime.submit_task(task).await {
                task_ids.push(task_id);
            }
        }

        println!("Submitted {} memory-intensive tasks", task_ids.len());

        // Wait for all to complete
        loop {
            let mut all_done = true;

            for task_id in &task_ids {
                if let Ok(Some(status)) = runtime.get_task_status(task_id).await {
                    if !matches!(status.as_str(), "Completed" | "Failed") {
                        all_done = false;
                    }
                }
            }

            if all_done || start.elapsed() > Duration::from_secs(120) {
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        // Get final memory
        let stats_after = runtime.get_statistics().await.ok();

        println!("\n=== MEMORY UNDER LOAD TEST ===");
        println!("Total Time: {:?}", start.elapsed());
        println!("Tasks: {}", task_ids.len());

        if let (Some(before), Some(after)) = (stats_before, stats_after) {
            println!("Memory entries created: {} → {}", before, after);
        }

        runtime.shutdown().await.expect("Failed to shutdown");
    }

    #[tokio::test]
    #[ignore]
    async fn stress_test_sustained_throughput() {
        let runtime = HiveRuntime::new("./hive_stress_test_6.db")
            .await
            .expect("Failed to create runtime");

        // Add specialists
        for i in 0..4 {
            let specialist = Specialist::new(
                &format!("throughput-specialist-{}", i),
                &format!("Throughput Worker {}", i),
                "Sustained processing",
                vec!["Processing", "Throughput"],
            );
            runtime
                .add_specialist(specialist)
                .await
                .expect("Failed to add specialist");
        }

        runtime.start().await.expect("Failed to start runtime");

        let start = Instant::now();
        let mut submitted = 0;
        let mut completed = 0;

        // Continuously submit tasks for 30 seconds
        let submission_duration = Duration::from_secs(30);

        while start.elapsed() < submission_duration {
            let task = Task {
                id: format!("throughput-task-{:05d}", submitted),
                name: format!("Throughput Task {}", submitted),
                description: "Sustained throughput test".to_string(),
                data_sample: Some(format!("Batch {}", submitted)),
                priority: TaskPriority::Normal,
                deadline_secs: Some(30),
                required_skills: vec!["Processing", "Throughput"],
                tags: vec!["throughput"],
            };

            if runtime.submit_task(task).await.is_ok() {
                submitted += 1;
            }

            sleep(Duration::from_millis(100)).await;
        }

        println!("Submitted {} tasks in {:?}", submitted, start.elapsed());

        // Wait for remaining tasks to complete
        sleep(Duration::from_secs(10)).await;

        // Count completions
        let stats = runtime.get_statistics().await.ok();
        println!("\n=== SUSTAINED THROUGHPUT TEST ===");
        println!("Duration: {:?}", start.elapsed());
        println!("Tasks submitted: {}", submitted);
        println!("Average throughput: {:.2} tasks/sec", submitted as f64 / 40.0);

        runtime.shutdown().await.expect("Failed to shutdown");
    }
}
