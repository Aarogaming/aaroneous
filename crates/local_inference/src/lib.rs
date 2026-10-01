use anyhow::{Result, anyhow};
use llama_gguf::engine::{Engine, EngineConfig};
use std::path::PathBuf;
use tracing::info;

pub struct LocalEngine {
    engine: Engine,
}

pub struct InferenceConfig {
    pub model_path: PathBuf,
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
}

/// Returns `Ok(())` if this crate was built with an explicit inference
/// backend feature enabled (`cpu`, `cuda`, `vulkan`, or `metal`), and a
/// clearly-named error otherwise.
///
/// `local_inference` never guesses a backend. Every one of `llama-gguf`'s
/// GPU feature flags crosses a native driver/API boundary that needs its own
/// isolated adapter and review (see the C68 native-boundary audit), and even
/// the CPU path must be a deliberate choice: `llama-gguf`'s own `cpu`
/// feature flag does not, today, gate any code path (its CPU backend module
/// is unconditionally compiled in), which means a caller that forgets to
/// select a backend would otherwise silently get CPU inference with no
/// record that anyone asked for it. Production configuration must select a
/// backend explicitly and fail clearly if it is unavailable — it must never
/// silently fall back to CPU or a mock. This check is what enforces that at
/// the one place both `LocalEngine::load` and `GGUFProvider::new` share.
pub fn ensure_backend_selected() -> Result<()> {
    let any_backend_selected = cfg!(feature = "cpu")
        || cfg!(feature = "cuda")
        || cfg!(feature = "vulkan")
        || cfg!(feature = "metal");

    if any_backend_selected {
        Ok(())
    } else {
        Err(anyhow!(
            "local_inference: no inference backend feature is enabled. Refusing to silently pick \
             one. Build with exactly one of the `cpu`, `cuda`, `vulkan`, or `metal` features \
             enabled (from `llm_gateway`, enable one of its `gguf-cpu`, `gguf-cuda`, \
             `gguf-vulkan`, or `gguf-metal` features, e.g. \
             `cargo build -p llm_gateway --features gguf-cpu`)."
        ))
    }
}

impl LocalEngine {
    pub fn load(config: &InferenceConfig) -> Result<Self> {
        ensure_backend_selected()?;

        let model_path_str = config.model_path.to_string_lossy().to_string();
        info!("local_inference: loading engine from {}", model_path_str);

        let engine_config = EngineConfig {
            model_path: model_path_str,
            temperature: config.temperature,
            top_p: config.top_p,
            ..Default::default()
        };

        let engine =
            Engine::load(engine_config).map_err(|e| anyhow!("Engine::load failed: {:?}", e))?;

        Ok(Self { engine })
    }

    pub fn generate(&mut self, prompt: &str, max_tokens: u32) -> Result<String> {
        self.engine
            .generate(prompt, max_tokens as usize)
            .map_err(|e| anyhow!("Inference failed: {:?}", e))
    }

    pub fn embed(&mut self, _text: &str) -> Result<Vec<f32>> {
        Err(anyhow!(
            "Embedding not natively supported by local_inference yet"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ensure_backend_selected` must track the crate's actual active
    /// feature set exactly, whatever it happens to be for this test run.
    #[test]
    fn backend_selection_matches_active_features() {
        let any_backend_selected = cfg!(feature = "cpu")
            || cfg!(feature = "cuda")
            || cfg!(feature = "vulkan")
            || cfg!(feature = "metal");
        assert_eq!(ensure_backend_selected().is_ok(), any_backend_selected);
    }

    /// With `local_inference`'s `default = []`, an ordinary `cargo test -p
    /// local_inference` (no `--features` passed) exercises exactly this
    /// path: building/running without any backend feature must produce a
    /// clear, named error — never a silent mock or a silently-selected CPU
    /// fallback.
    #[cfg(not(any(
        feature = "cpu",
        feature = "cuda",
        feature = "vulkan",
        feature = "metal"
    )))]
    #[test]
    fn load_without_backend_feature_fails_with_named_error() {
        let config = InferenceConfig {
            model_path: PathBuf::from("nonexistent-model.gguf"),
            max_tokens: 16,
            temperature: 0.7,
            top_p: 0.95,
        };

        match LocalEngine::load(&config) {
            Ok(_) => panic!(
                "LocalEngine::load must fail when no backend feature is enabled, not silently \
                 succeed or fall back to a mock"
            ),
            Err(err) => assert!(
                err.to_string()
                    .contains("no inference backend feature is enabled"),
                "expected the named backend-selection error, got: {err}"
            ),
        }
    }
}
