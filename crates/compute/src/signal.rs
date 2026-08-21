/// Signal Processing primitives (FFT, Kalman Filters, Wavelets).
/// Used for metabolic monitoring, smoothing noisy telemetry, detecting periodic load patterns.
/// Basic Cooley-Tukey FFT (power-of-2 length required)
pub fn fft_basic(input: &[f64]) -> anyhow::Result<Vec<f64>> {
    let n = input.len();
    if n == 0 {
        return Ok(vec![]);
    }
    if n == 1 {
        return Ok(input.to_vec());
    }
    if n & (n - 1) != 0 {
        return Ok(input.to_vec());
    } // Not power of 2

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
    Ok(real
        .iter()
        .zip(imag.iter())
        .map(|(r, i)| (r.powi(2) + i.powi(2)).sqrt())
        .collect())
}

/// Industrial SIMD-accelerated FFT using RustFFT (arbitrary length)
pub fn fft_industrial(input: &[f64]) -> anyhow::Result<Vec<f64>> {
    use rustfft::num_complex::Complex;
    use rustfft::FftPlanner;

    if input.is_empty() {
        return Ok(vec![]);
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(input.len());

    let mut buffer: Vec<Complex<f64>> = input.iter().map(|&x| Complex { re: x, im: 0.0 }).collect();
    fft.process(&mut buffer);

    Ok(buffer.iter().map(|c| c.norm()).collect())
}

/// Calculate Power Spectral Density (PSD) from FFT magnitudes
pub fn power_spectral_density(magnitudes: &[f64]) -> Vec<f64> {
    let n = magnitudes.len() as f64;
    if n == 0.0 {
        return vec![];
    }
    magnitudes.iter().map(|m| (m * m) / n).collect()
}

/// Simple 1D Kalman filter step with process noise
pub fn kalman_step(measurement: f64, estimate: f64, error_est: f64, error_meas: f64) -> (f64, f64) {
    let process_noise = 0.05;
    let p_pred = error_est + process_noise;
    let gain = p_pred / (p_pred + error_meas);
    let new_est = estimate + gain * (measurement - estimate);
    let new_err = (1.0 - gain) * p_pred;
    (new_est, new_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_and_psd() {
        let signal = vec![1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let mag = fft_basic(&signal).unwrap();
        assert_eq!(mag.len(), 8);

        let psd = power_spectral_density(&mag);
        assert_eq!(psd.len(), 8);
        assert!(psd[0] >= 0.0);
    }

    #[test]
    fn test_kalman_filter_convergence() {
        let mut est: f64 = 0.0;
        let mut err: f64 = 1.0;
        let measurement: f64 = 10.0;

        for _ in 0..30 {
            let (new_est, new_err) = kalman_step(measurement, est, err, 0.5);
            est = new_est;
            err = new_err;
        }

        // Kalman filter should converge closely to 10.0
        assert!((est - 10.0f64).abs() < 0.1);
        assert!(err < 0.5);
    }

    #[test]
    fn test_fft_industrial() {
        let signal = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]; // non-power-of-two length
        let mag = fft_industrial(&signal).unwrap();
        assert_eq!(mag.len(), 7);
        assert!(mag[0] > 0.0);
    }
}
