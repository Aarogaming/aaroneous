#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ExecutionPhase {
    /// Nominal operation - full throughput
    Nominal,
    /// Reduced polling rate due to drift/jitter
    BackedOff,
    /// Critical state - SMT fence engaged
    Critical,
}

/// Adaptive Duty Cycle Governor - adjusts execution phases based on prediction residuals.
pub struct AdaptiveDutyGovernor {
    /// Current phase state
    current_phase: ExecutionPhase,
    /// Residual error threshold for backing off (0.05)
    jitter_threshold: f32,
    /// Critical residual threshold (0.20)
    critical_threshold: f32,
}

impl Default for AdaptiveDutyGovernor {
    fn default() -> Self {
        Self {
            current_phase: ExecutionPhase::Nominal,
            jitter_threshold: 0.05,
            critical_threshold: 0.20,
        }
    }
}

impl AdaptiveDutyGovernor {
    /// Adjust duty cycle based on residual error metric.
    /// 
    /// # Arguments
    /// * `residual_error` - Normalized prediction residual (0.0 = perfect, higher = worse)
    /// 
    /// # Returns
    /// Current execution phase AFTER adjustment
    pub fn adjust_duty_cycle(&mut self, residual_error: f32) -> ExecutionPhase {
        if residual_error >= self.critical_threshold {
            self.current_phase = ExecutionPhase::Critical;
        } else if residual_error >= self.jitter_threshold {
            self.current_phase = ExecutionPhase::BackedOff;
        } else {
            self.current_phase = ExecutionPhase::Nominal;
        }
        
        self.current_phase
    }

    /// Get current phase state (by value to avoid move issues).
    pub fn get_phase(self) -> ExecutionPhase {
        self.current_phase
    }

    // No need for peek_phase - just return by value in adjust_duty_cycle

    /// Reset to nominal phase (for recovery after Critical state).
    pub fn reset_to_nominal(&mut self) {
        self.current_phase = ExecutionPhase::Nominal;
    }
}
// impl Default for ExecutionPhase removed - use Nominal instead
