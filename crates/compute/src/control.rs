//! crates/compute/src/control.rs
//! Control Theory primitives (PID with anti-windup, Low-Pass Filtering, Bang-Bang).
//! Used for dynamic metabolic throttling, resource allocation, and system stability.

use anyhow::Result;

/// Comprehensive PID Controller with anti-windup clamping, output bounds, and derivative filtering.
#[derive(Debug, Clone)]
pub struct PidController {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub integral: f64,
    pub prev_error: f64,
    pub filtered_derivative: f64,
    pub derivative_filter_alpha: f64,
    pub output_limits: Option<(f64, f64)>,
    pub integral_limits: Option<(f64, f64)>,
}

impl PidController {
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
            filtered_derivative: 0.0,
            derivative_filter_alpha: 1.0, // Default: no filtering
            output_limits: None,
            integral_limits: None,
        }
    }

    /// Sets minimum and maximum allowable control output.
    pub fn with_output_limits(mut self, min: f64, max: f64) -> Self {
        self.output_limits = Some((min, max));
        self
    }

    /// Sets anti-windup limits on the accumulated integral term.
    pub fn with_integral_limits(mut self, min: f64, max: f64) -> Self {
        self.integral_limits = Some((min, max));
        self
    }

    /// Sets derivative low-pass filter coefficient (0.0 < alpha <= 1.0).
    pub fn with_derivative_filter(mut self, alpha: f64) -> Self {
        self.derivative_filter_alpha = alpha.clamp(0.01, 1.0);
        self
    }

    /// Resets internal controller state (integral and previous error).
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
        self.filtered_derivative = 0.0;
    }

    /// Advances the controller by one unit timestep (backwards-compatible).
    pub fn step(&mut self, setpoint: f64, measured: f64) -> f64 {
        self.step_timed(setpoint, measured, 1.0)
    }

    /// Advances the controller with an explicit timestep `dt_seconds`.
    pub fn step_timed(&mut self, setpoint: f64, measured: f64, dt_seconds: f64) -> f64 {
        let dt = dt_seconds.max(1e-6);
        let error = setpoint - measured;

        // Integral accumulation with anti-windup clamping
        self.integral += error * dt;
        if let Some((min_i, max_i)) = self.integral_limits {
            self.integral = self.integral.clamp(min_i, max_i);
        }

        // Filtered derivative calculation: d = alpha * raw_d + (1 - alpha) * filtered_d
        let raw_derivative = (error - self.prev_error) / dt;
        self.filtered_derivative = self.derivative_filter_alpha * raw_derivative
            + (1.0 - self.derivative_filter_alpha) * self.filtered_derivative;
        self.prev_error = error;

        let mut output = self.kp * error + self.ki * self.integral + self.kd * self.filtered_derivative;

        // Output saturation
        if let Some((min_out, max_out)) = self.output_limits {
            output = output.clamp(min_out, max_out);
        }

        output
    }
}

/// First-order discrete low-pass filter: y[n] = alpha * x[n] + (1 - alpha) * y[n-1].
#[derive(Debug, Clone)]
pub struct FirstOrderLowPassFilter {
    pub alpha: f64,
    pub state: f64,
    pub initialized: bool,
}

impl FirstOrderLowPassFilter {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            state: 0.0,
            initialized: false,
        }
    }

    pub fn step(&mut self, input: f64) -> f64 {
        if !self.initialized {
            self.state = input;
            self.initialized = true;
        } else {
            self.state = self.alpha * input + (1.0 - self.alpha) * self.state;
        }
        self.state
    }
}

/// Bang-Bang Controller with hysteresis deadband.
#[derive(Debug, Clone)]
pub struct BangBangController {
    pub hysteresis: f64,
    pub high_output: f64,
    pub low_output: f64,
    pub current_output: f64,
}

impl BangBangController {
    pub fn new(hysteresis: f64, low_output: f64, high_output: f64) -> Self {
        Self {
            hysteresis: hysteresis.max(0.0),
            high_output,
            low_output,
            current_output: low_output,
        }
    }

    pub fn step(&mut self, setpoint: f64, measured: f64) -> f64 {
        let error = setpoint - measured;
        if error > self.hysteresis {
            self.current_output = self.high_output;
        } else if error < -self.hysteresis {
            self.current_output = self.low_output;
        }
        self.current_output
    }
}

/// PID step from input [setpoint, measured, kp, ki, kd, integral, prev_error] (backwards-compatible).
pub fn pid_step(input: &[f64]) -> Result<Vec<f64>> {
    if input.len() < 5 {
        return Ok(vec![]);
    }
    let mut pid = PidController::new(input[2], input[3], input[4]);
    pid.integral = if input.len() > 5 { input[5] } else { 0.0 };
    pid.prev_error = if input.len() > 6 { input[6] } else { 0.0 };
    let output = pid.step(input[0], input[1]);
    Ok(vec![output, pid.integral, pid.prev_error])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_basic_and_limits() {
        let mut pid = PidController::new(2.0, 0.5, 0.1)
            .with_output_limits(-10.0, 10.0)
            .with_integral_limits(-5.0, 5.0);

        let out = pid.step(10.0, 0.0);
        // Error = 10 -> P = 20, I = 5, D = 1.0 -> Sum 26 -> Clamped to 10.0
        assert_eq!(out, 10.0);
        assert_eq!(pid.integral, 5.0); // Clamped to integral limit
    }

    #[test]
    fn test_low_pass_filter() {
        let mut filter = FirstOrderLowPassFilter::new(0.5);
        let s0 = filter.step(10.0);
        assert_eq!(s0, 10.0); // Initialized directly

        let s1 = filter.step(20.0);
        assert_eq!(s1, 15.0); // 0.5*20 + 0.5*10 = 15.0
    }

    #[test]
    fn test_bang_bang_hysteresis() {
        let mut bb = BangBangController::new(1.0, 0.0, 100.0);
        assert_eq!(bb.step(10.0, 5.0), 100.0); // Error = 5 > 1 -> High
        assert_eq!(bb.step(10.0, 9.5), 100.0); // Error = 0.5 within deadband -> Stays High
        assert_eq!(bb.step(10.0, 12.0), 0.0);  // Error = -2 < -1 -> Low
    }
}
