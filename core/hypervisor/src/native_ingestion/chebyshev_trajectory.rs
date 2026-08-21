use crate::native_ingestion::ScreenCoordinate;

/// Polynomial trajectory fitting via least-squares with monomial basis.
///
/// Reduces a sequence of mouse movement waypoints to 6 compressed float
/// coefficients using degree-2 polynomial approximation per axis.
/// Removes hand tremors and processing lag by discarding high-frequency
/// noise above the polynomial degree cutoff.
///
/// Input: array of (x, y) screen coordinates in [0.0, 1.0] fractional space.
/// Output: `[f32; 6]` coefficients: [cx0, cy0, cx1, cy1, cx2, cy2]
///   x(t) = cx0 + cx1*t + cx2*t²
///   y(t) = cy0 + cy1*t + cy2*t²
/// Number of compressed coefficients.
pub const CHEBYSHEV_DEGREE: usize = 6;

/// Fit degree-2 polynomials to the x and y trajectories using linear
/// least-squares via the normal equations.
pub fn fit_chebyshev(waypoints: &[ScreenCoordinate]) -> [f32; 6] {
    let n = waypoints.len();
    if n == 0 {
        return [0.0f32; 6];
    }

    // Design matrix: [1, t, t²] evaluated at each normalized waypoint position.
    // t ranges from 0.0 to 1.0 across the sequence.
    // For n points: t_i = i / (n-1), or 0 for n=1
    // Solve: (A^T A) c = A^T y  via direct construction for degree-2

    let s0 = n as f32; // Σ 1
    let mut s1 = 0.0f32; // Σ t_i
    let mut s2 = 0.0f32; // Σ t_i²
    let mut s3 = 0.0f32; // Σ t_i³
    let mut s4 = 0.0f32; // Σ t_i⁴

    let mut bx0 = 0.0f32; // Σ x_i
    let mut bx1 = 0.0f32; // Σ x_i * t_i
    let mut bx2 = 0.0f32; // Σ x_i * t_i²

    let mut by0 = 0.0f32;
    let mut by1 = 0.0f32;
    let mut by2 = 0.0f32;

    let denom = (n - 1).max(1) as f32;

    for (i, wp) in waypoints.iter().enumerate() {
        let t = i as f32 / denom;
        let t2 = t * t;
        let t3 = t2 * t;
        let t4 = t3 * t;

        s1 += t;
        s2 += t2;
        s3 += t3;
        s4 += t4;

        bx0 += wp.x;
        bx1 += wp.x * t;
        bx2 += wp.x * t2;

        by0 += wp.y;
        by1 += wp.y * t;
        by2 += wp.y * t2;
    }

    // Normal equations for degree-2: A^T A c = A^T b
    // | s0  s1  s2 | | c0 |   | bx0 |
    // | s1  s2  s3 | | c1 | = | bx1 |
    // | s2  s3  s4 | | c2 |   | bx2 |
    //
    // Solve using Cramer's rule or direct formula for 3x3.

    let det = s0 * (s2 * s4 - s3 * s3) - s1 * (s1 * s4 - s2 * s3) + s2 * (s1 * s3 - s2 * s2);

    let (cx0, cx1, cx2) = if det.abs() > 1e-12 {
        // Cramer's rule for x coefficients
        let det_c0 =
            bx0 * (s2 * s4 - s3 * s3) - s1 * (bx1 * s4 - bx2 * s3) + s2 * (bx1 * s3 - bx2 * s2);
        let det_c1 =
            s0 * (bx1 * s4 - bx2 * s3) - bx0 * (s1 * s4 - s2 * s3) + s2 * (s1 * bx2 - s2 * bx1);
        let det_c2 =
            s0 * (s2 * bx2 - s3 * bx1) - s1 * (s1 * bx2 - s2 * bx1) + bx0 * (s1 * s3 - s2 * s2);
        (det_c0 / det, det_c1 / det, det_c2 / det)
    } else {
        (bx0 / s0, 0.0, 0.0)
    };

    let (cy0, cy1, cy2) = if det.abs() > 1e-12 {
        let det_c0 =
            by0 * (s2 * s4 - s3 * s3) - s1 * (by1 * s4 - by2 * s3) + s2 * (by1 * s3 - by2 * s2);
        let det_c1 =
            s0 * (by1 * s4 - by2 * s3) - by0 * (s1 * s4 - s2 * s3) + s2 * (s1 * by2 - s2 * by1);
        let det_c2 =
            s0 * (s2 * by2 - s3 * by1) - s1 * (s1 * by2 - s2 * by1) + by0 * (s1 * s3 - s2 * s2);
        (det_c0 / det, det_c1 / det, det_c2 / det)
    } else {
        (by0 / s0, 0.0, 0.0)
    };

    [cx0, cy0, cx1, cy1, cx2, cy2]
}

/// Evaluate the fitted polynomial at position `t` in [0.0, 1.0].
pub fn evaluate_chebyshev(coeffs: &[f32; 6], t: f32) -> ScreenCoordinate {
    let t_clamped = t.clamp(0.0, 1.0);
    let t2 = t_clamped * t_clamped;
    let x = coeffs[0] + coeffs[2] * t_clamped + coeffs[4] * t2;
    let y = coeffs[1] + coeffs[3] * t_clamped + coeffs[5] * t2;
    ScreenCoordinate {
        x: x.clamp(0.0, 1.0),
        y: y.clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_waypoints() {
        let coeffs = fit_chebyshev(&[]);
        assert_eq!(coeffs, [0.0f32; 6]);
    }

    #[test]
    fn test_linear_trajectory() {
        let waypoints: Vec<ScreenCoordinate> = (0..10)
            .map(|i| {
                let t = i as f32 / 9.0;
                ScreenCoordinate { x: t, y: t }
            })
            .collect();

        let coeffs = fit_chebyshev(&waypoints);
        // Linear path: x = t, so coefficients should be [0, 0, 1, 1, 0, 0]
        assert!((coeffs[0] - 0.0).abs() < 0.01, "cx0={}", coeffs[0]);
        assert!((coeffs[2] - 1.0).abs() < 0.01, "cx1={}", coeffs[2]);
        assert!((coeffs[4]).abs() < 0.01, "cx2={}", coeffs[4]); // zero quadratic term

        for i in 0..10 {
            let t = i as f32 / 9.0;
            let pt = evaluate_chebyshev(&coeffs, t);
            assert!((pt.x - t).abs() < 0.01, "x at t={}: {} vs {}", t, pt.x, t);
            assert!((pt.y - t).abs() < 0.01, "y at t={}: {} vs {}", t, pt.y, t);
        }
    }

    #[test]
    fn test_constant_trajectory() {
        let waypoints: Vec<ScreenCoordinate> = (0..5)
            .map(|_| ScreenCoordinate { x: 0.5, y: 0.3 })
            .collect();

        let coeffs = fit_chebyshev(&waypoints);
        // Constant: c0 = 0.5, c1 = 0, c2 = 0
        assert!((coeffs[0] - 0.5).abs() < 0.001);
        assert!((coeffs[1] - 0.3).abs() < 0.001);

        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let pt = evaluate_chebyshev(&coeffs, t);
            assert!((pt.x - 0.5).abs() < 0.001, "x at t={}: {} vs 0.5", t, pt.x);
            assert!((pt.y - 0.3).abs() < 0.001, "y at t={}: {} vs 0.3", t, pt.y);
        }
    }

    #[test]
    fn test_parabolic_trajectory() {
        let waypoints: Vec<ScreenCoordinate> = (0..10)
            .map(|i| {
                let t = i as f32 / 9.0;
                ScreenCoordinate {
                    x: t * t,
                    y: 1.0 - t * t,
                }
            })
            .collect();

        let coeffs = fit_chebyshev(&waypoints);
        assert!((coeffs[0] - 0.0).abs() < 0.01); // x = t² → c0≈0, c1≈0, c2≈1
        assert!((coeffs[4] - 1.0).abs() < 0.1);

        for i in 0..10 {
            let t = i as f32 / 9.0;
            let pt = evaluate_chebyshev(&coeffs, t);
            assert!(
                (pt.x - t * t).abs() < 0.02,
                "x at t={}: {} vs {}",
                t,
                pt.x,
                t * t
            );
        }
    }
}
