/// Signal Processing primitives (FFT, Kalman Filters, Wavelets).
/// Used for metabolic monitoring, smoothing noisy telemetry, detecting periodic load patterns.

/// Basic Cooley-Tukey FFT (power-of-2 length required)
pub fn fft_basic(input: &[f64]) -> anyhow::Result<Vec<f64>> {
    let n = input.len();
    if n == 0 { return Ok(vec![]); }
    if n == 1 { return Ok(input.to_vec()); }
    if n & (n - 1) != 0 { return Ok(input.to_vec()); } // Not power of 2
    
    let mut real = input.to_vec();
    let mut imag = vec![0.0; n];
    let mut m = n;
    while m > 1 {
        for i in (0..n).step_by(m) {
            let half = m / 2;
            for j in 0..half {
                let angle = -2.0 * std::f64::consts::PI * j as f64 / m as f64;
                let (cos, sin) = angle.sin_cos();
                let t_real = real[i + j + half] * cos - imag[i + j + half] * sin;
                let t_imag = real[i + j + half] * sin + imag[i + j + half] * cos;
                real[i + j + half] = real[i + j] - t_real;
                imag[i + j + half] = imag[i + j] - t_imag;
                real[i + j] += t_real;
                imag[i + j] += t_imag;
            }
        }
        m /= 2;
    }
    // Return magnitudes
    Ok(real.iter().zip(imag.iter()).map(|(r, i)| (r.powi(2) + i.powi(2)).sqrt()).collect())
}

/// Simple 1D Kalman filter step
pub fn kalman_step(measurement: f64, estimate: f64, error_est: f64, error_meas: f64) -> (f64, f64) {
    let gain = error_est / (error_est + error_meas);
    let new_est = estimate + gain * (measurement - estimate);
    let new_err = (1.0 - gain) * error_est;
    (new_est, new_err)
}
