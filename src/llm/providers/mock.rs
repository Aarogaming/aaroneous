// Mock LLM Provider
// Used for testing without calling real APIs

use crate::llm::types::*;
use anyhow::Result;
use async_trait::async_trait;
use tracing::debug;

#[derive(Debug, Clone, Default)]
pub struct MockProvider;

#[async_trait]
impl super::LLMProvider for MockProvider {
    async fn analyze_task(&self, context: &TaskAnalysisContext) -> Result<TaskAnalysis> {
        debug!("Mock: Analyzing task {}", context.task_id);

        Ok(TaskAnalysis {
            task_id: context.task_id.clone(),
            analysis_type: match context.file_type.as_str() {
                "csv" => "data_analysis".to_string(),
                "json" => "data_structure_analysis".to_string(),
                "gguf" => "model_analysis".to_string(),
                _ => "general_analysis".to_string(),
            },
            complexity: Complexity::Moderate,
            recommended_approach: format!(
                "Use {} skills to analyze this file",
                context.specialist_skills.join(" + ")
            ),
            estimated_time_minutes: 30,
            confidence_percentage: 85,
            suggested_collaborators: vec!["Circe".to_string()], // Default suggestion
            potential_risks: vec!["Data quality issues".to_string()],
            reasoning: "Mock analysis for testing".to_string(),
        })
    }

    async fn find_collaborators(
        &self,
        specialist: &SpecialistContext,
    ) -> Result<Vec<CollaboratorSuggestion>> {
        debug!("Mock: Finding collaborators for {}", specialist.name);

        let suggestions = match specialist.name.as_str() {
            "Merlin" => vec![
                CollaboratorSuggestion {
                    specialist_name: "Circe".to_string(),
                    reason: "Complementary analysis skills".to_string(),
                    relevance_score: 0.9,
                    complementary_skills: vec!["statistical_analysis".to_string()],
                },
            ],
            "Ariel" => vec![
                CollaboratorSuggestion {
                    specialist_name: "Hephaestus".to_string(),
                    reason: "Complementary tool skills".to_string(),
                    relevance_score: 0.8,
                    complementary_skills: vec!["system_integration".to_string()],
                },
            ],
            _ => vec![
                CollaboratorSuggestion {
                    specialist_name: "Odin".to_string(),
                    reason: "Leadership coordination".to_string(),
                    relevance_score: 0.7,
                    complementary_skills: vec!["orchestration".to_string()],
                },
            ],
        };

        Ok(suggestions)
    }

    async fn generate_plan(
        &self,
        task: &TaskAnalysis,
        specialist: &SpecialistContext,
    ) -> Result<ExecutionPlan> {
        debug!("Mock: Generating plan for {}", specialist.name);

        Ok(ExecutionPlan {
            task_id: task.task_id.clone(),
            specialist_name: specialist.name.clone(),
            steps: vec![
                PlanStep {
                    sequence: 1,
                    description: "Read and validate input".to_string(),
                    estimated_time_minutes: 5,
                    required_skills: vec!["basic_analysis".to_string()],
                    checkpoints: vec!["Data loaded successfully".to_string()],
                },
                PlanStep {
                    sequence: 2,
                    description: "Analyze data".to_string(),
                    estimated_time_minutes: 20,
                    required_skills: task.suggested_collaborators.clone(),
                    checkpoints: vec!["Analysis complete".to_string()],
                },
                PlanStep {
                    sequence: 3,
                    description: "Generate results".to_string(),
                    estimated_time_minutes: 5,
                    required_skills: vec!["synthesis".to_string()],
                    checkpoints: vec!["Results ready".to_string()],
                },
            ],
            total_estimated_time: 30,
            success_probability: 0.85,
            reasoning: "Standard analysis workflow".to_string(),
        })
    }

    async fn analyze_failure(&self, failure: &FailureContext) -> Result<FailureAnalysis> {
        debug!("Mock: Analyzing failure for {}", failure.task_id);

        Ok(FailureAnalysis {
            root_cause: "Unexpected data format".to_string(),
            explanation: "The file had unexpected structure".to_string(),
            prevention_strategy: "Always validate schema first".to_string(),
            recovery_approach: "Use lenient parsing mode".to_string(),
            new_strategy: "Try strict parsing, fall back to lenient".to_string(),
            confidence_percentage: 80,
        })
    }

    async fn explain_skill(
        &self,
        skill_name: &str,
        specialist: &SpecialistContext,
    ) -> Result<SkillExplanation> {
        debug!("Mock: Explaining skill {} for {}", skill_name, specialist.name);

        Ok(SkillExplanation {
            skill_name: skill_name.to_string(),
            description: format!("This is the {} skill", skill_name),
            use_cases: vec![
                "Use case 1".to_string(),
                "Use case 2".to_string(),
            ],
            example: "Here's an example of how to use it".to_string(),
            how_to_improve: "Practice using it on different tasks".to_string(),
            synergies_with: vec!["other_skill1".to_string()],
        })
    }

    async fn chat(
        &self,
        system_prompt: &str,
        user_message: &str,
        domain: &str,
    ) -> Result<String> {
        debug!("Mock chat: domain={} user='{:.60}'", domain, user_message);
        // Route through generate_design with the domain hint so the structured
        // mock responses fire (task_orchestration → JSON task graph, etc.)
        let ctx = crate::llm::types::DesignContext {
            intent: format!("[SYSTEM: {}]\n\n{}", system_prompt, user_message),
            style_hints: vec![domain.to_string()],
            constraints: vec![format!("domain: {}", domain)],
            variants_requested: 1,
            approved_examples: vec![],
            rejected_examples: vec![],
        };
        let generation = self.generate_design(&ctx).await?;
        Ok(generation.variants.into_iter().next()
            .map(|v| v.description)
            .unwrap_or_else(|| format!("[mock] no response for domain {}", domain)))
    }

    async fn generate_design(&self, context: &DesignContext) -> Result<DesignGeneration> {
        debug!("Mock: Generating {} design variant(s) for '{}'",
               context.variants_requested, context.intent);

        // If the intent contains a domain-specific system prompt (from generate_domain_response),
        // return structured JSON appropriate for that domain rather than UI design variants.
        // This makes Odin, Argus, Merlin etc. return parseable structured output in mock mode.
        if let Some(domain) = context.style_hints.first() {
            match domain.as_str() {
                "task_orchestration" | "guild_coordination" | "intent_routing" => {
                    let intent = context.intent
                        .splitn(2, "\n\n").nth(1)
                        .unwrap_or(&context.intent)
                        .chars().take(100).collect::<String>();
                    let intent_lower = intent.to_lowercase();

                    // Route t2 to the correct sovereign based on intent keywords
                    // instead of always routing to Ariel (UI design) which is wrong
                    // for code/security/build intents.
                    let (t2_sovereign, t2_action) = if intent_lower.contains("security")
                        || intent_lower.contains("audit") || intent_lower.contains("vulnerab")
                        || intent_lower.contains("exploit") || intent_lower.contains("scan")
                    {
                        ("Argus", format!("Security audit: {}", &intent[..intent.len().min(60)]))
                    } else if intent_lower.contains("code") || intent_lower.contains("review")
                        || intent_lower.contains("rust") || intent_lower.contains("function")
                        || intent_lower.contains("bug") || intent_lower.contains("refactor")
                    {
                        ("Merlin", format!("Code analysis: {}", &intent[..intent.len().min(60)]))
                    } else if intent_lower.contains("build") || intent_lower.contains("deploy")
                        || intent_lower.contains("ci") || intent_lower.contains("pipeline")
                        || intent_lower.contains("docker") || intent_lower.contains("compile")
                    {
                        ("Hephaestus", format!("Build planning: {}", &intent[..intent.len().min(60)]))
                    } else if intent_lower.contains("design") || intent_lower.contains("ui")
                        || intent_lower.contains("ux") || intent_lower.contains("interface")
                        || intent_lower.contains("layout") || intent_lower.contains("visual")
                    {
                        ("Ariel", format!("Design generation: {}", &intent[..intent.len().min(60)]))
                    } else {
                        // General intent → Merlin for synthesis
                        ("Merlin", format!("Execute primary work on: {}", &intent[..intent.len().min(60)]))
                    };

                    let json = format!(
                        r#"{{"mock":true,"source":"odin_mock","tasks":[{{"id":"t1","content":"Research and gather context for: {}","assign_to":"Merlin","priority":"High","deps":[]}},{{"id":"t2","content":"{}","assign_to":"{}","priority":"Normal","deps":["t1"]}},{{"id":"t3","content":"Archive results and update DNA Bank","assign_to":"Dionysus","priority":"Low","deps":["t2"]}}],"note":"Enable --features llama-gguf for real Odin task decomposition"}}"#,
                        intent, t2_action, t2_sovereign
                    );
                    return Ok(DesignGeneration {
                        intent: context.intent.clone(),
                        variants: vec![DesignVariant {
                            title: "Task Decomposition".into(),
                            description: json,
                            colors: vec![],
                            typography: String::new(),
                            layout: "task_dag".into(),
                            confidence: 0.85,
                            reasoning: "Odin guild decomposition (mock)".into(),
                        }],
                        tokens_used: 0,
                        batch_confidence: 0.85,
                    });
                }
                "security_audit" | "secrets_management" | "vulnerability_scanning" => {
                    let intent = context.intent.chars().take(80).collect::<String>();
                    let json = format!(r#"{{"mock":true,"source":"argus_mock","target":"{}","findings":[{{"severity":"Info","description":"Mock — no real security analysis performed","remediation":"Enable --features llama-gguf for real Argus scanning"}}],"overall_risk":"Unknown","note":"This is a mock output."}}"#, intent);
                    return Ok(DesignGeneration {
                        intent: context.intent.clone(),
                        variants: vec![DesignVariant {
                            title: "Security Audit".into(),
                            description: json,
                            colors: vec![], typography: String::new(), layout: "security_report".into(),
                            confidence: 0.3, reasoning: "Argus audit (mock — not a real scan)".into(),
                        }],
                        tokens_used: 0, batch_confidence: 0.3,
                    });
                }
                "research" | "knowledge_synthesis" | "external_research" => {
                    let intent = context.intent.chars().take(80).collect::<String>();
                    let json = format!(r#"{{"mock":true,"source":"merlin_mock","query":"{}","summary":"Mock placeholder — no real synthesis","key_findings":["This is a mock response","Enable --features llama-gguf for real Merlin research"],"confidence":0.3}}"#, intent);
                    return Ok(DesignGeneration {
                        intent: context.intent.clone(),
                        variants: vec![DesignVariant {
                            title: "Research Synthesis".into(), description: json,
                            colors: vec![], typography: String::new(), layout: "research_report".into(),
                            confidence: 0.3, reasoning: "Merlin synthesis (mock)".into(),
                        }],
                        tokens_used: 0, batch_confidence: 0.3,
                    });
                }
                "fabrication" | "maintenance" | "infrastructure" | "construction" => {
                    let intent = context.intent.chars().take(100).collect::<String>();
                    let json = format!(r#"{{"mock":true,"source":"hephaestus_mock","task":"{}","plan":[{{"step":1,"action":"Assess requirements","status":"planned"}},{{"step":2,"action":"Identify dependencies","status":"planned"}},{{"step":3,"action":"Generate build manifest","status":"planned"}}],"note":"Enable --features llama-gguf for real Hephaestus build planning"}}"#, intent);
                    return Ok(DesignGeneration {
                        intent: context.intent.clone(),
                        variants: vec![DesignVariant {
                            title: "Fabrication Plan".into(), description: json,
                            colors: vec![], typography: String::new(), layout: "fabrication_plan".into(),
                            confidence: 0.3, reasoning: "Hephaestus plan (mock)".into(),
                        }],
                        tokens_used: 0, batch_confidence: 0.3,
                    });
                }
                "mesh_sync" | "p2p" | "multi_device" => {
                    let json = r#"{"mock":true,"source":"hermes_mock","devices":[],"conflicts":[],"bandwidth_mbps":0,"status":"no_p2p_attached","note":"Enable --features p2p-iroh for real Hermes mesh sync"}"#;
                    return Ok(DesignGeneration {
                        intent: context.intent.clone(),
                        variants: vec![DesignVariant {
                            title: "Mesh Sync".into(), description: json.into(),
                            colors: vec![], typography: String::new(), layout: "sync_status".into(),
                            confidence: 0.3, reasoning: "Hermes sync (mock)".into(),
                        }],
                        tokens_used: 0, batch_confidence: 0.3,
                    });
                }
                "spatial" | "ar_vr" | "physical_digital" => {
                    let intent = context.intent.chars().take(80).collect::<String>();
                    let json = format!(
                        r#"{{"mock":true,"source":"kami_mock","intent":"{}","spatial":{{"anchor_count":1,"device":"simulated","frame_rate_fps":60,"ar_available":false,"render_mode":"simulated"}},"anchors":[{{"prototype_id":"synth-mock","design_variant":"{}","landmark":"arm-reach-default","model":"/models/synth.glb","scale":1.0,"ar_available":false,"source":"intent_derived"}}],"note":"Enable --features ar-openxr for real Kami AR rendering"}}"#,
                        intent, intent
                    );
                    return Ok(DesignGeneration {
                        intent: context.intent.clone(),
                        variants: vec![DesignVariant {
                            title: "Spatial Anchor".into(), description: json,
                            colors: vec![], typography: String::new(), layout: "spatial_manifest".into(),
                            confidence: 0.3, reasoning: "Kami spatial (mock — no AR runtime)".into(),
                        }],
                        tokens_used: 0, batch_confidence: 0.3,
                    });
                }
                "biometric" | "human_state" | "user_adaptation" => {
                    let json = r#"{"mock":true,"source":"wen_mock","state":"unknown","stress":0.5,"fatigue":0.3,"readiness":70,"recommendation":"continue","defer_interruptions":false,"note":"Enable biometric sensor or --features biometric-ble for real Wen readings"}"#;
                    return Ok(DesignGeneration {
                        intent: context.intent.clone(),
                        variants: vec![DesignVariant {
                            title: "Biometric State".into(), description: json.into(),
                            colors: vec![], typography: String::new(), layout: "biometric_state".into(),
                            confidence: 0.3, reasoning: "Wen biometric (mock)".into(),
                        }],
                        tokens_used: 0, batch_confidence: 0.3,
                    });
                }
                "memory_consolidation" | "archival" => {
                    let json = r#"{"mock":true,"source":"dionysus_mock","consolidated_events":0,"patterns_discovered":0,"dna_bank_size_mb":0,"note":"Dionysus accumulates patterns across sessions — submit more intents to build memory"}"#;
                    return Ok(DesignGeneration {
                        intent: context.intent.clone(),
                        variants: vec![DesignVariant {
                            title: "Memory Consolidation".into(), description: json.into(),
                            colors: vec![], typography: String::new(), layout: "consolidation_report".into(),
                            confidence: 0.4, reasoning: "Dionysus archival (mock)".into(),
                        }],
                        tokens_used: 0, batch_confidence: 0.4,
                    });
                }
                _ => {} // Fall through to default design generation below
            }
        }

        let count = context.variants_requested.min(3).max(1);
        let mut variants = Vec::with_capacity(count);

        let palettes = [
            vec!["#6366F1".to_string(), "#A5B4FC".to_string(), "#1E1B4B".to_string()],
            vec!["#10B981".to_string(), "#6EE7B7".to_string(), "#065F46".to_string()],
            vec!["#F59E0B".to_string(), "#FDE68A".to_string(), "#92400E".to_string()],
        ];
        let layouts = ["single-column", "card-grid", "sidebar-nav"];
        let typography = ["Inter, sans-serif", "DM Sans, sans-serif", "Sora, sans-serif"];

        for i in 0..count {
            let style_hint = context.style_hints.first()
                .map(|s| s.as_str())
                .unwrap_or("modern");

            variants.push(DesignVariant {
                title: format!("Variant {} – {} {:?}", i + 1, style_hint, context.intent),
                description: format!(
                    "A {} design for '{}' with {} constraints. Uses {} layout \
                     with a {} color palette. Optimized for readability and conversion.",
                    style_hint,
                    context.intent,
                    if context.constraints.is_empty() { "no specific".to_string() }
                    else { context.constraints.join(", ") },
                    layouts[i % layouts.len()],
                    if context.style_hints.contains(&"dark-theme".to_string()) { "dark" } else { "light" }
                ),
                colors: palettes[i % palettes.len()].clone(),
                typography: typography[i % typography.len()].to_string(),
                layout: layouts[i % layouts.len()].to_string(),
                confidence: 0.7 + (i as f32 * 0.05),
                reasoning: format!(
                    "Based on {} approved examples and avoiding {} rejected patterns. \
                     Style hint '{}' maps to variant {}.",
                    context.approved_examples.len(),
                    context.rejected_examples.len(),
                    style_hint,
                    i + 1
                ),
            });
        }

        Ok(DesignGeneration {
            intent: context.intent.clone(),
            variants,
            tokens_used: 0, // Mock has no token cost
            batch_confidence: 0.75,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_provider_creation() {
        let provider = MockProvider::default();
        let cloned = provider.clone();
        assert_eq!(format!("{:?}", cloned), format!("{:?}", provider));
    }

    // Integration tests in main llm::tests verify provider functionality
}
