use anyhow::Result;
use chimera_marionette_loop::chimera::{ChimeraEngine, PatchProposal};
use chimera_marionette_loop::marionette::{HidCommand, MarionetteHost, VisualObservation};
use chimera_marionette_loop::ChimeraMarionetteLoop;

struct MockChimera;
#[async_trait::async_trait]
impl ChimeraEngine for MockChimera {
    async fn synthesize_patch(&self, _: &str, _: &str) -> Result<PatchProposal> {
        unimplemented!()
    }
    async fn apply_patch(&self, _: &PatchProposal) -> Result<()> {
        unimplemented!()
    }
}

struct MockMarionette;
#[async_trait::async_trait]
impl MarionetteHost for MockMarionette {
    async fn pull_visual_perception(&self) -> Result<VisualObservation> {
        unimplemented!()
    }
    async fn inject_hid_event(&self, _: HidCommand) -> Result<()> {
        unimplemented!()
    }
}

#[tokio::test]
async fn test_shadow_sandbox_isolation() -> Result<()> {
    let loop_engine = ChimeraMarionetteLoop::new(Box::new(MockChimera), Box::new(MockMarionette))?;
    let sample_code = b"fn main() { let x = 42; }";

    let mut synapse = nervous_system::shared_memory::SynapseState::default();
    let success = loop_engine
        .run_sandboxed("safe_test.rs", sample_code, &mut synapse)
        .await?;
    assert!(success, "Sandboxed compilation check failed");

    // Verify file is strictly inside .sab/shadow/
    let shadow_file = std::env::current_dir()?
        .join(".sab")
        .join("shadow")
        .join("safe_test.rs");
    assert!(
        shadow_file.exists(),
        "Shadow file was not created in .sab/shadow/"
    );

    // Clean up
    let _ = std::fs::remove_file(shadow_file);
    Ok(())
}
