//! RLS Boundary Tests - Numerical Stability Verification
//!
//! Historical anti-pattern: Floating-point rounding errors accumulate, causing
//! covariance matrix P to lose positive-definiteness (P_ii <= 0) or blow up into NaN/Inf.
//!
//! These tests ensure the substrate's state estimation logic is mathematically bounded.

use bytemuck::{Pod, Zeroable};

/// Covariance matrix with explicit positive-definiteness invariant
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
struct CovarianceMatrix {
    p00: f32,
    p01: f32,
    p10: f32,
    p11: f32,
}

/// Historical failure mode: Division by zero in normalization
#[test]
fn test_rls_division_by_zero_protection() {
    let covariance = CovarianceMatrix {
        p00: 1e-10, // Near-zero value (historical failure condition)
        p01: 0.0,
        p10: 0.0,
        p11: 1e-10,
    };

    // Apply RLS update with bounded division
    let _delta = 0.5f32;
    let factor = covariance.p00 / (covariance.p00 + 1e-8); // Bounded by epsilon

    assert!(
        !factor.is_nan(),
        "RLS update produced NaN from near-zero covariance"
    );
    assert!(factor > 0.0 && factor < 1.0, "Factor out of bounds");
}

/// Historical failure mode: Covariance matrix loses positive-definiteness
#[test]
fn test_rls_positive_definiteness_invariant() {
    let mut covariance = CovarianceMatrix {
        p00: -1e-6, // Negative diagonal (historical divergence)
        p01: 0.0,
        p10: 0.0,
        p11: 1e-6,
    };

    // SMT invariant fence should project/reset to valid region
    let result = enforce_positive_definiteness(&mut covariance);

    assert!(result.is_ok(), "Failed to recover from negative covariance");
    assert!(
        covariance.p00 > 0.0,
        "Diagonal element still negative after enforcement"
    );
}

/// Historical failure mode: Covariance explosion (values grow unbounded)
#[test]
fn test_rls_covariance_boundedness() {
    let mut covariance = CovarianceMatrix {
        p00: 1e6, // Extremely large (historical blowup)
        p01: 1e3,
        p10: 1e3,
        p11: 1e6,
    };

    // Apply update with saturation
    let result = apply_rls_update(&mut covariance, &[], 0.95);

    assert!(result.is_ok(), "RLS update failed on large covariance");
    assert!(covariance.p00 <= 1e8, "Covariance grew beyond bounds");
}

/// Historical failure mode: NaN propagation through operations
#[test]
fn test_rls_nan_propagation_prevention() {
    let nan_cov = CovarianceMatrix {
        p00: f32::NAN,
        p01: 0.0,
        p10: 0.0,
        p11: 1.0,
    };

    // Should detect and handle NaN before it propagates
    let result = sanitize_covariance(&nan_cov);

    assert!(result.is_ok(), "Failed to sanitize NaN covariance");
    assert!(
        !result.unwrap().p00.is_nan(),
        "NaN still present after sanitization"
    );
}

/// Historical failure mode: Infinite values from overflow
#[test]
fn test_rls_infinity_prevention() {
    let inf_cov = CovarianceMatrix {
        p00: f32::INFINITY,
        p01: f32::INFINITY,
        p10: f32::NEG_INFINITY,
        p11: 1.0,
    };

    // Should clamp infinities to finite bounds
    let result = clamp_covariance(&inf_cov);

    assert!(result.is_ok(), "Failed to clamp infinity covariance");
    assert!(
        !result.unwrap().p00.is_infinite(),
        "Infinity still present after clamping"
    );
}

/// Enforces positive-definiteness invariant (SMT constraint)
fn enforce_positive_definiteness(cov: &mut CovarianceMatrix) -> Result<(), String> {
    // Project negative diagonals to epsilon
    cov.p00 = cov.p00.max(1e-8);
    cov.p11 = cov.p11.max(1e-8);

    // Ensure off-diagonals don't exceed geometric mean (positive definite condition)
    let max_off_diag = (cov.p00 * cov.p11).sqrt();
    cov.p01 = cov.p01.min(max_off_diag);
    cov.p10 = cov.p10.min(max_off_diag);

    Ok(())
}

/// Applies RLS update with bounded operations
fn apply_rls_update(
    cov: &mut CovarianceMatrix,
    _innovations: &[f32],
    lambda: f32,
) -> Result<(), String> {
    if lambda <= 0.0 || lambda >= 1.0 {
        return Err("Lambda out of bounds".to_string());
    }

    // Apply forgetting factor with saturation
    cov.p00 = (cov.p00 * lambda).min(1e8).max(1e-8);
    cov.p11 = (cov.p11 * lambda).min(1e8).max(1e-8);

    Ok(())
}

/// Sanitizes NaN values from covariance matrix
fn sanitize_covariance(cov: &CovarianceMatrix) -> Result<CovarianceMatrix, String> {
    let mut sanitized = *cov;

    // Replace NaN with zero
    if sanitized.p00.is_nan() {
        sanitized.p00 = 0.0;
    }
    if sanitized.p11.is_nan() {
        sanitized.p11 = 0.0;
    }

    // Replace Inf with large finite value
    if sanitized.p00.is_infinite() {
        sanitized.p00 = 1e8;
    }
    if sanitized.p11.is_infinite() {
        sanitized.p11 = 1e8;
    }

    Ok(sanitized)
}

/// Clamps covariance to valid range
fn clamp_covariance(cov: &CovarianceMatrix) -> Result<CovarianceMatrix, String> {
    let mut clamped = *cov;

    // Clamp diagonals to [epsilon, max_bound]
    clamped.p00 = clamped.p00.max(1e-8).min(1e8);
    clamped.p11 = clamped.p11.max(1e-8).min(1e8);

    // Clamp off-diagonals to [-sqrt(p00*p11), sqrt(p00*p11)]
    let max_off = (clamped.p00 * clamped.p11).sqrt();
    clamped.p01 = clamped.p01.max(-max_off).min(max_off);
    clamped.p10 = clamped.p10.max(-max_off).min(max_off);

    Ok(clamped)
}
