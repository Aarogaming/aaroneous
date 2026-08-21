/// Control Theory primitives (PID, MPC).
/// Used for dynamic metabolic throttling, resource allocation, and system stability.

#[derive(Debug, Clone)]
pub struct PidController {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub integral: f64,
    pub prev_error: f64,
}

impl PidController {
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
        }
    }

    pub fn step(&mut self, setpoint: f64, measured: f64) -> f64 {
        let error = setpoint - measured;
        self.integral += error;
        let derivative = error - self.prev_error;
        self.prev_error = error;
        self.kp * error + self.ki * self.integral + self.kd * derivative
    }
}

/// PID step from input [setpoint, measured, kp, ki, kd, integral, prev_error]
pub fn pid_step(input: &[f64]) -> anyhow::Result<Vec<f64>> {
    if input.len() < 5 {
        return Ok(vec![]);
    }
    let mut pid = PidController::new(input[2], input[3], input[4]);
    pid.integral = if input.len() > 5 { input[5] } else { 0.0 };
    pid.prev_error = if input.len() > 6 { input[6] } else { 0.0 };
    let output = pid.step(input[0], input[1]);
    Ok(vec![output, pid.integral, pid.prev_error])
}
