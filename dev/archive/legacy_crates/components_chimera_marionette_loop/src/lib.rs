pub mod sandbox;
pub mod chimera;
pub mod marionette;

use anyhow::{Result};
use serde::{Deserialize, Serialize};
use self::sandbox::ShadowSandbox;
pub use self::chimera::{ChimeraEngine, PatchProposal};
pub use self::marionette::{MarionetteHost};
use nervous_system::shared_memory::SynapseState;

/// Loop metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct LoopMetadata {
    pub iterations: usize,
    pub total_time_ms: u64,
    pub errors_corrected: usize,
}

/// Chimera-Marionette Loop
pub struct ChimeraMarionetteLoop {
    sandbox: ShadowSandbox,
    chimera: Box<dyn ChimeraEngine>,
    marionette: Box<dyn MarionetteHost>,
}

impl ChimeraMarionetteLoop {
    /// Create a new ChimeraMarionetteLoop with isolated shadow sandbox
    pub fn new(chimera: Box<dyn ChimeraEngine>, marionette: Box<dyn MarionetteHost>) -> Result<Self> {
        Ok(Self {
            sandbox: ShadowSandbox::new()?,
            chimera,
            marionette,
        })
    }
    
    /// Run sandboxed compilation and verification loop, feeding back dopamine/penalties to SynapseState
    pub async fn run_sandboxed(&mut self, file_name: &str, content: &[u8], synapse: &mut SynapseState) -> Result<bool> {
        tracing::info!(target: "chimera_loop", file = %file_name, "Starting Sandboxed Chimera Verification Loop");
        
        let start_time = std::time::Instant::now();
        let _shadow_path = self.sandbox.write_shadow_file(file_name, content)?;
        
        // 1. Synthesize patch if necessary
        let proposal = self.chimera.synthesize_patch(&String::from_utf8_lossy(content), "initial_audit").await?;
        
        // 2. Execute syntax check
        let (success, _output) = self.sandbox.execute_syntax_check(file_name)?;
        let _elapsed = start_time.elapsed().as_millis() as u64;
        
        // Use marionette to perhaps visualize the process if needed
        let _ = self.marionette.pull_visual_perception().await;

        // Feedback Loop: Update SynapseState based on success
        if success {
            // Reward: Increase integrity and understanding
            synapse.integrity_score = synapse.integrity_score.saturating_add(5).min(100);
            synapse.understanding_score = synapse.understanding_score.saturating_add(2).min(100);
            tracing::info!(target: "chimera_loop", "Success! Dopamine signal injected. Integrity: {}", synapse.integrity_score);
        } else {
            // Penalty: Decrease integrity and trigger potential safety lock
            synapse.integrity_score = synapse.integrity_score.saturating_sub(10);
            tracing::warn!(target: "chimera_loop", "Failure! Penalty signal injected. Integrity: {}", synapse.integrity_score);
            // Apply patch if confidence is high
            if proposal.confidence > 0.8 {
                let _ = self.chimera.apply_patch(&proposal).await;
            }
        }

        Ok(success)
    }
}
