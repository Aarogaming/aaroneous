// Integration tests for specialist memory system

#[cfg(test)]
mod memory_integration_tests {
    use crate::{
        SpecialistMemory, MemoryEntry, MemoryType, Confidence, MemorySource,
        DecisionRecord, Strategy, Goal, GoalStatus, MemoryStats,
    };

    #[test]
    fn test_memory_entry_creation_and_retrieval() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        let entry = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Lesson,
            "Pattern Recognition".to_string(),
            "Learned to identify patterns in data".to_string(),
        )
        .with_tags(vec!["patterns".to_string(), "data".to_string()])
        .with_confidence(Confidence::High);

        memory.record_memory(entry);

        assert_eq!(memory.memories.len(), 1);
        let retrieved = memory.search_memories("patterns");
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].title, "Pattern Recognition");
    }

    #[test]
    fn test_decision_recording_and_outcome() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        let decision = DecisionRecord::new(
            "spec-1".to_string(),
            "Use clustering algorithm".to_string(),
            "Data shows natural groupings".to_string(),
        )
        .add_outcome(true, "Successfully clustered data with 95% accuracy".to_string());

        memory.record_decision(decision);

        let recent = memory.get_recent_decisions(10);
        assert_eq!(recent.len(), 1);
        assert!(recent[0].outcome.is_some());
        assert!(recent[0].outcome.as_ref().unwrap().success);
    }

    #[test]
    fn test_strategy_effectiveness_tracking() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        let strategy = Strategy::new(
            "spec-1".to_string(),
            "Quick Analysis".to_string(),
            "Fast data processing approach".to_string(),
        )
        .add_step(
            "Scan data for patterns".to_string(),
            "Identify key structures".to_string(),
        )
        .add_step(
            "Apply algorithm".to_string(),
            "Process identified patterns".to_string(),
        );

        memory.save_strategy(strategy);

        let saved = memory.strategies.values().next().unwrap();
        assert_eq!(saved.steps.len(), 2);
        assert_eq!(saved.success_count, 0);
        assert_eq!(saved.failure_count, 0);

        // Track usage
        let mut strat = saved.clone();
        strat = strat.record_success().record_success().record_failure();

        memory.save_strategy(strat);

        let updated = memory.strategies.values().next().unwrap();
        assert_eq!(updated.success_count, 2);
        assert_eq!(updated.failure_count, 1);
        assert_eq!(updated.effectiveness_score, 2.0 / 3.0); // ~0.667
    }

    #[test]
    fn test_goal_lifecycle() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        let goal = Goal::new(
            "spec-1".to_string(),
            "Master Data Analysis".to_string(),
            "Improve data processing capabilities".to_string(),
        )
        .activate();

        assert_eq!(goal.status, GoalStatus::Active);

        memory.save_goal(goal);
        let active_goals = memory.get_active_goals();
        assert_eq!(active_goals.len(), 1);

        let mut goal = active_goals[0].clone();
        goal = goal.complete();
        memory.save_goal(goal);

        let completed = memory.goals.values().next().unwrap();
        assert_eq!(completed.status, GoalStatus::Completed);
        assert_eq!(completed.progress_percentage, 100);
    }

    #[test]
    fn test_goal_with_blockers() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        let goal = Goal::new(
            "spec-1".to_string(),
            "Learn Advanced ML".to_string(),
            "Develop machine learning expertise".to_string(),
        )
        .activate()
        .add_blocker("Missing GPU resources".to_string())
        .add_blocker("Need additional training data".to_string());

        memory.save_goal(goal);

        // Blocked goals are not considered "active" in the query
        let active_goals = memory.get_active_goals();
        assert_eq!(active_goals.len(), 0); // Blocked goals aren't active

        // But they are stored in memory
        let all_goals = memory.goals.values().collect::<Vec<_>>();
        assert_eq!(all_goals.len(), 1);
        assert_eq!(all_goals[0].blockers.len(), 2);
        assert_eq!(all_goals[0].status, GoalStatus::Blocked);
    }

    #[test]
    fn test_memory_stats_calculation() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        // Add various memory entries
        for i in 0..5 {
            let entry = MemoryEntry::new(
                "spec-1".to_string(),
                MemoryType::Lesson,
                format!("Lesson {}", i),
                "Description".to_string(),
            );
            memory.record_memory(entry);
        }

        // Add strategies
        for i in 0..3 {
            let strategy = Strategy::new(
                "spec-1".to_string(),
                format!("Strategy {}", i),
                "Description".to_string(),
            );
            memory.save_strategy(strategy);
        }

        // Add goals
        let goal = Goal::new(
            "spec-1".to_string(),
            "Test Goal".to_string(),
            "Testing".to_string(),
        )
        .activate();
        memory.save_goal(goal);

        let stats = memory.get_memory_stats();

        assert_eq!(stats.total_memories, 5);
        assert_eq!(stats.lessons, 5);
        assert_eq!(stats.strategies, 3);
        assert_eq!(stats.active_goals, 1);
        assert!(stats.memory_health > 0.0);
        assert!(stats.memory_health <= 1.0);
    }

    #[test]
    fn test_memory_search_and_filtering() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        // Create memories with different tags
        for task_type in &["data_processing", "pattern_recognition", "optimization"] {
            let entry = MemoryEntry::new(
                "spec-1".to_string(),
                MemoryType::Lesson,
                format!("Lesson for {}", task_type),
                "Description".to_string(),
            )
            .with_tags(vec![task_type.to_string()]);

            memory.record_memory(entry);
        }

        // Search by tag
        let data_processing = memory.search_memories("data_processing");
        assert_eq!(data_processing.len(), 1);
        assert!(data_processing[0].title.contains("data_processing"));

        let pattern_memories = memory.search_memories("pattern_recognition");
        assert_eq!(pattern_memories.len(), 1);
    }

    #[test]
    fn test_strategy_retrieval_by_task_type() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        // Create strategies for different task types
        let mut strategy1 = Strategy::new(
            "spec-1".to_string(),
            "Analysis Strategy".to_string(),
            "For analysis tasks".to_string(),
        );
        strategy1.applicable_to = vec!["data_analysis".to_string()];
        strategy1.effectiveness_score = 0.8;

        let mut strategy2 = Strategy::new(
            "spec-1".to_string(),
            "Cleanup Strategy".to_string(),
            "For data cleanup".to_string(),
        );
        strategy2.applicable_to = vec!["data_cleanup".to_string()];
        strategy2.effectiveness_score = 0.9;

        memory.save_strategy(strategy1);
        memory.save_strategy(strategy2);

        // Get strategies for specific task type
        let analysis_strategies = memory.get_strategies_for_task("data_analysis");
        assert_eq!(analysis_strategies.len(), 1);
        assert_eq!(analysis_strategies[0].name, "Analysis Strategy");

        // Get best strategy
        let best = memory.get_best_strategy("data_cleanup");
        assert!(best.is_some());
        assert_eq!(best.unwrap().effectiveness_score, 0.9);
    }

    #[test]
    fn test_multiple_memory_types() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        // Add different memory types
        let lesson = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Lesson,
            "A Lesson".to_string(),
            "Description".to_string(),
        );

        let strategy_entry = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Strategy,
            "A Strategy".to_string(),
            "Description".to_string(),
        );

        let reflection_entry = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Reflection,
            "A Reflection".to_string(),
            "Description".to_string(),
        );

        memory.record_memory(lesson);
        memory.record_memory(strategy_entry);
        memory.record_memory(reflection_entry);

        // Filter by type
        let lessons = memory.get_memories_by_type(MemoryType::Lesson);
        assert_eq!(lessons.len(), 1);
        assert_eq!(lessons[0].memory_type, MemoryType::Lesson);

        let reflections = memory.get_memories_by_type(MemoryType::Reflection);
        assert_eq!(reflections.len(), 1);
        assert_eq!(reflections[0].memory_type, MemoryType::Reflection);
    }

    #[test]
    fn test_memory_confidence_levels() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        let low_conf = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Lesson,
            "Low Confidence".to_string(),
            "Description".to_string(),
        )
        .with_confidence(Confidence::Low);

        let high_conf = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Lesson,
            "High Confidence".to_string(),
            "Description".to_string(),
        )
        .with_confidence(Confidence::High);

        memory.record_memory(low_conf);
        memory.record_memory(high_conf);

        let all_memories: Vec<_> = memory.memories.values().collect();
        assert_eq!(all_memories.len(), 2);

        let has_low = all_memories.iter().any(|m| m.confidence == Confidence::Low);
        let has_high = all_memories.iter().any(|m| m.confidence == Confidence::High);

        assert!(has_low);
        assert!(has_high);
    }

    #[test]
    fn test_memory_source_tracking() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        let from_experience = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Lesson,
            "From Experience".to_string(),
            "Description".to_string(),
        )
        .with_source(MemorySource::Experience);

        let from_llm = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Lesson,
            "From LLM".to_string(),
            "Description".to_string(),
        )
        .with_source(MemorySource::LLMReasoning);

        memory.record_memory(from_experience);
        memory.record_memory(from_llm);

        let all: Vec<_> = memory.memories.values().collect();
        let exp_source = all.iter().filter(|m| m.source == MemorySource::Experience).count();
        let llm_source = all.iter().filter(|m| m.source == MemorySource::LLMReasoning).count();

        assert_eq!(exp_source, 1);
        assert_eq!(llm_source, 1);
    }

    #[test]
    fn test_related_memories_linking() {
        let mut memory = SpecialistMemory::new("spec-1".to_string());

        let memory1 = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Lesson,
            "Memory 1".to_string(),
            "Description".to_string(),
        );
        let id1 = memory1.id.clone();

        let mut memory2 = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Lesson,
            "Memory 2".to_string(),
            "Description".to_string(),
        );
        memory2.related_memories.push(id1.clone());

        memory.record_memory(memory1);
        memory.record_memory(memory2);

        let mem2 = memory.memories.values().find(|m| m.title == "Memory 2").unwrap();
        assert_eq!(mem2.related_memories.len(), 1);
        assert_eq!(mem2.related_memories[0], id1);
    }

    #[test]
    fn test_memory_relevance_decay() {
        let mut entry = MemoryEntry::new(
            "spec-1".to_string(),
            MemoryType::Lesson,
            "Test".to_string(),
            "Description".to_string(),
        );

        assert_eq!(entry.relevance_score, 1.0);

        // Simulate relevance decay
        entry.relevance_score = 0.9;
        entry.usage_count += 1;

        assert_eq!(entry.usage_count, 1);
        assert!(entry.relevance_score < 1.0);
    }
}
