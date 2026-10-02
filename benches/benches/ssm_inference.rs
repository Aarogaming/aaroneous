use compute::si_solid_state::{SiOnlineLearner, SolidStateSiContainer};
use compute::si_ssm::SiSsmConfig;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Mock tokenizer + embedding + softmax overhead simulating an autoregressive LLM pass
fn traditional_token_llm_mock(input_text: &str) -> String {
    // 1. Tokenize (O(N) string processing mock)
    let tokens: Vec<&str> = input_text.split_whitespace().collect();

    // 2. Mock embedding lookup & transformer layers overhead (~10ms simulated latency for a small model)
    // We do some floating point math to simulate the compute without sleeping
    let mut state = 0.0f32;
    for _ in 0..10_000 {
        for token in &tokens {
            state += (token.len() as f32).sin().cos();
        }
    }

    // 3. Output detokenize
    format!("Action: {}", state)
}

fn ssm_inference_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("inference_latency");

    // Initialize Continuous SSI Model
    let config = SiSsmConfig::default();
    let container = SolidStateSiContainer::new("Bench-SSI", config).unwrap();
    let mut learner = SiOnlineLearner::new(container, false).unwrap();

    // Continuous Input Vector (Simulating Sensor Fusion / Latent State)
    let input_vector = vec![0.5f32; 1024];

    group.bench_function("aaroneous_ssi_tier3_reflex", |b| {
        b.iter(|| {
            // Memory-mapped weights -> State-space recurrence -> Latent state -> Action
            learner
                .forward_adapted_step(black_box(&input_vector))
                .unwrap()
        })
    });

    let input_text = "sensor reading 0.5 across all 1024 channels proceed to next action sequence";

    group.bench_function("traditional_llm_autoregressive_mock", |b| {
        b.iter(|| {
            // Text -> Tokens -> Embeddings -> Transformer Layers -> Logits -> Tokens -> Text
            traditional_token_llm_mock(black_box(input_text))
        })
    });

    group.finish();
}

criterion_group!(benches, ssm_inference_benchmark);
criterion_main!(benches);
