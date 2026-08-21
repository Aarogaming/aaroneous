use anyhow::Result;
use chimera_marionette_loop::ChimeraMarionetteLoop;
use chimera_marionette_loop::chimera::TreeSitterChimera;
use chimera_marionette_loop::marionette::NativeMarionette;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Initializing Chimera Isolated Sandbox Runtime");

    let chimera = TreeSitterChimera::new();
    let marionette = NativeMarionette::new();
    let mut loop_engine = ChimeraMarionetteLoop::new(Box::new(chimera), Box::new(marionette))?;
    
    // Test sandboxed compilation check
    let test_code = b"fn main() { println!(\"Sandbox Verification\"); }";
    let mut synapse = nervous_system::shared_memory::SynapseState::default();
    let success = loop_engine.run_sandboxed("verification_test.rs", test_code, &mut synapse).await?;

    tracing::info!(success = success, "Sandbox verification complete");

    Ok(())
}
