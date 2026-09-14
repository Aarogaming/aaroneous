//! Scale-Invariant Dynamics & Symplectic Solvers
//!
//! Provides multi-domain Bond-Graph Duality ($P(t) = e(t) \cdot f(t)$),
//! Symplectic Hamiltonian integration ($d\mathcal{H}/dt \approx 0$),
//! and dimensionless invariant evaluations across physical domains.

/// Numerical and physical computation errors.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ComputeError {
    /// Time step parameter is invalid (must be strictly positive and finite).
    InvalidTimeStep,
    /// Physical parameter is invalid (mass, stiffness, or compliance must be strictly positive and finite).
    InvalidParameter,
    /// Numerical divergence or instability detected (state coordinates are non-finite).
    NumericalInstability,
    /// System energy diverged beyond acceptable conservation bounds.
    EnergyDivergence,
}

impl core::fmt::Display for ComputeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidTimeStep => {
                f.pad("Invalid integration time step (dt must be finite and > 0.0)")
            }
            Self::InvalidParameter => {
                f.pad("Physical parameter must be positive and finite")
            }
            Self::NumericalInstability => {
                f.pad("Numerical instability detected (state coordinates are non-finite)")
            }
            Self::EnergyDivergence => {
                f.pad("Hamiltonian energy diverged beyond conservation bounds")
            }
        }
    }
}

impl std::error::Error for ComputeError {}

/// Physical domain classification for bond-graph energy exchange.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PhysicalDomain {
    Translational,
    Rotational,
    Electrical,
    Hydraulic,
    Thermal,
}

/// Generalized conjugate power variables (Effort $e$ and Flow $f$).
///
/// In Bond-Graph duality, power transmitted across any energetic port is
/// uniformly defined as $P(t) = e(t) \cdot f(t)$.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EffortFlowPair {
    pub effort: f64,
    pub flow: f64,
}

impl EffortFlowPair {
    /// Creates a new Effort-Flow conjugate pair.
    pub const fn new(effort: f64, flow: f64) -> Self {
        Self { effort, flow }
    }

    /// Computes instantaneous power transmitted across the energetic bond: $P = e \cdot f$.
    pub fn instantaneous_power(&self) -> f64 {
        self.effort * self.flow
    }
}

/// Unified contract for physical dynamical systems integrated via symplectic algorithms.
pub trait DynamicalSystem {
    /// Advances system state forward by `dt` seconds using symplectic integration.
    fn step_symplectic(&mut self, dt: f64) -> Result<(), crate::ComputeError>;

    /// Evaluates the total Hamiltonian energy $\mathcal{H}(\mathbf{q}, \mathbf{p}) = T(\mathbf{p}) + V(\mathbf{q})$.
    fn total_energy(&self) -> f64;

    /// Verifies Hamiltonian energy conservation within the specified tolerance.
    fn check_hamiltonian_conservation(&self, initial_energy: f64, tolerance: f64) -> bool {
        (self.total_energy() - initial_energy).abs() <= tolerance
    }

    /// Returns the physical domain of the dynamical system.
    fn domain(&self) -> PhysicalDomain;
}

/// Generalized harmonic oscillator modeling multi-domain dual systems.
///
/// Models translational spring-mass ($F = -kx$), rotational inertia-spring ($\tau = -\kappa\theta$),
/// LC tank circuits ($V = q/C, \dot{\lambda} = -V$), or hydraulic accumulator-inertance duals.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HarmonicOscillatorDual {
    pub domain: PhysicalDomain,
    /// Generalized inertia / mass / inductance / fluid inertance ($I$-element).
    pub m: f64,
    /// Generalized stiffness / spring constant / inverse capacitance / hydraulic elastance ($C^{-1}$).
    pub k: f64,
    /// Generalized coordinate / displacement / position / charge / volume ($q$).
    pub q: f64,
    /// Generalized momentum / linear momentum / flux / pressure momentum ($p$).
    pub p: f64,
}

/// Type alias for the reference harmonic oscillator implementation.
pub type HarmonicOscillator = HarmonicOscillatorDual;

impl HarmonicOscillatorDual {
    /// Creates a new generalized harmonic oscillator.
    pub fn new(domain: PhysicalDomain, m: f64, k: f64, q: f64, p: f64) -> Self {
        Self { domain, m, k, q, p }
    }

    /// Translational spring-mass oscillator ($m$ = mass [kg], $k$ = spring constant [N/m]).
    pub fn translational(mass: f64, spring_k: f64, position: f64, momentum: f64) -> Self {
        Self::new(PhysicalDomain::Translational, mass, spring_k, position, momentum)
    }

    /// Rotational inertia-torsion oscillator ($J$ = moment of inertia [kg*m^2], $\kappa$ = torsion constant [N*m/rad]).
    pub fn rotational(inertia: f64, torsion_k: f64, angle: f64, angular_momentum: f64) -> Self {
        Self::new(PhysicalDomain::Rotational, inertia, torsion_k, angle, angular_momentum)
    }

    /// Electrical LC tank circuit ($L$ = inductance [H], $C^{-1}$ = inverse capacitance [1/F]).
    pub fn electrical(inductance: f64, inv_capacitance: f64, charge: f64, flux: f64) -> Self {
        Self::new(PhysicalDomain::Electrical, inductance, inv_capacitance, charge, flux)
    }

    /// Hydraulic accumulator-inertance oscillator ($I_f$ = fluid inertance [kg/m^4], $C_h^{-1}$ = elastance [Pa/m^3]).
    pub fn hydraulic(inertance: f64, elastance: f64, volume: f64, pressure_momentum: f64) -> Self {
        Self::new(PhysicalDomain::Hydraulic, inertance, elastance, volume, pressure_momentum)
    }

    /// Natural undamped angular frequency $\omega_0 = \sqrt{k / m}$ [rad/s].
    pub fn natural_frequency(&self) -> f64 {
        (self.k / self.m).sqrt()
    }

    /// Dimensionless Courant-Friedrichs-Lewy (CFL) number $\Omega = \omega_0 \cdot \Delta t$.
    ///
    /// For symplectic Euler integration, the scheme is conditionally stable when $\Omega < 2.0$.
    pub fn courant_number(&self, dt: f64) -> f64 {
        self.natural_frequency() * dt
    }

    /// Dimensionless damping ratio $\zeta = \frac{R}{2\sqrt{km}}$ for a given dissipative resistance $R$.
    pub fn damping_ratio(&self, resistance_r: f64) -> f64 {
        resistance_r / (2.0 * (self.k * self.m).sqrt())
    }

    /// Dimensionless quality factor $Q = \frac{\sqrt{km}}{R}$.
    pub fn quality_factor(&self, resistance_r: f64) -> f64 {
        (self.k * self.m).sqrt() / resistance_r
    }

    /// Dimensionless energy conservation ratio $E(t) / E_0$.
    pub fn energy_ratio(&self, initial_energy: f64) -> f64 {
        if initial_energy == 0.0 {
            1.0
        } else {
            self.total_energy() / initial_energy
        }
    }

    /// Returns the instantaneous Effort-Flow pair for the current system state.
    ///
    /// Effort $e = k \cdot q$, Flow $f = p / m$.
    pub fn state_effort_flow(&self) -> EffortFlowPair {
        EffortFlowPair::new(self.k * self.q, self.p / self.m)
    }
}

impl DynamicalSystem for HarmonicOscillatorDual {
    /// Steps the system using symplectic Euler integration:
    ///
    /// $$q(t + \Delta t) = q(t) + \Delta t \cdot \frac{p(t)}{m}$$
    /// $$p(t + \Delta t) = p(t) - \Delta t \cdot k \cdot q(t + \Delta t)$$
    ///
    /// Preserves the canonical symplectic 2-form $dq \wedge dp$ with zero secular energy drift.
    fn step_symplectic(&mut self, dt: f64) -> Result<(), crate::ComputeError> {
        if dt <= 0.0 || !dt.is_finite() {
            return Err(crate::ComputeError::InvalidTimeStep);
        }
        if self.m <= 0.0 || self.k < 0.0 || !self.m.is_finite() || !self.k.is_finite() {
            return Err(crate::ComputeError::InvalidParameter);
        }

        // q(t + dt) = q(t) + dt * p(t) / m
        self.q += dt * (self.p / self.m);

        // p(t + dt) = p(t) - dt * k * q(t + dt)
        self.p -= dt * self.k * self.q;

        if !self.q.is_finite() || !self.p.is_finite() {
            return Err(crate::ComputeError::NumericalInstability);
        }

        Ok(())
    }

    /// Computes total Hamiltonian energy:
    ///
    /// $$\mathcal{H}(q, p) = T(p) + V(q) = \frac{p^2}{2m} + \frac{1}{2} k q^2$$
    fn total_energy(&self) -> f64 {
        0.5 * (self.p * self.p) / self.m + 0.5 * self.k * (self.q * self.q)
    }

    fn domain(&self) -> PhysicalDomain {
        self.domain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effort_flow_power_duality() {
        // Translational: Force (N) * Velocity (m/s) = Power (W)
        let translational = EffortFlowPair::new(50.0, 2.0);
        assert_eq!(translational.instantaneous_power(), 100.0);

        // Rotational: Torque (N*m) * Angular Velocity (rad/s) = Power (W)
        let rotational = EffortFlowPair::new(25.0, 4.0);
        assert_eq!(rotational.instantaneous_power(), 100.0);

        // Electrical: Voltage (V) * Current (A) = Power (W)
        let electrical = EffortFlowPair::new(120.0, 0.5);
        assert_eq!(electrical.instantaneous_power(), 60.0);

        // Hydraulic: Pressure (Pa) * Volume Flow (m^3/s) = Power (W)
        let hydraulic = EffortFlowPair::new(100_000.0, 0.002);
        assert_eq!(hydraulic.instantaneous_power(), 200.0);

        // Thermal: Temperature difference (K) * Entropy Flow Rate (W/K) = Heat Flow (W)
        let thermal = EffortFlowPair::new(300.0, 0.25);
        assert_eq!(thermal.instantaneous_power(), 75.0);

        // Verify state_effort_flow produces matching duality
        let osc = HarmonicOscillatorDual::translational(2.0, 50.0, 1.5, 6.0);
        let pair = osc.state_effort_flow();
        // Effort e = k * q = 50.0 * 1.5 = 75.0 N
        // Flow f = p / m = 6.0 / 2.0 = 3.0 m/s
        assert_eq!(pair.effort, 75.0);
        assert_eq!(pair.flow, 3.0);
        assert_eq!(pair.instantaneous_power(), 225.0);
    }

    #[test]
    fn test_symplectic_energy_conservation() {
        let mut osc = HarmonicOscillatorDual::translational(1.0, 10.0, 1.0, 0.0);
        let initial_energy = osc.total_energy();
        // E_0 = 0.5 * k * q^2 = 0.5 * 10.0 * 1.0^2 = 5.0 J
        assert_eq!(initial_energy, 5.0);

        let dt = 0.01;
        let tolerance = 0.25; // Symplectic Euler energy fluctuation is bounded by O(dt) with ZERO secular drift

        for _ in 0..1000 {
            osc.step_symplectic(dt).expect("Symplectic step must succeed");
            assert!(
                osc.check_hamiltonian_conservation(initial_energy, tolerance),
                "Energy drifted outside conservation bounds: current = {}, initial = {}",
                osc.total_energy(),
                initial_energy
            );
        }

        // Verify that after 1,000 steps (~5 full oscillations), energy has not drifted away
        let final_energy = osc.total_energy();
        let drift = (final_energy - initial_energy).abs();
        assert!(
            drift <= tolerance,
            "Total energy secular drift after 1,000 steps: {}",
            drift
        );

        // Dimensionless energy ratio remains centered around 1.0
        let ratio = osc.energy_ratio(initial_energy);
        assert!((ratio - 1.0).abs() < 0.05);
    }

    #[test]
    fn test_domain_constructors_and_dimensionless_invariants() {
        let osc = HarmonicOscillatorDual::rotational(2.0, 50.0, 0.5, 0.0);
        assert_eq!(osc.domain(), PhysicalDomain::Rotational);

        // Natural frequency w0 = sqrt(k / m) = sqrt(50 / 2) = 5.0 rad/s
        assert_eq!(osc.natural_frequency(), 5.0);

        // Courant number CFL = w0 * dt = 5.0 * 0.02 = 0.1 (< 2.0, stable)
        assert_eq!(osc.courant_number(0.02), 0.10);

        // Damping ratio zeta = R / (2 * sqrt(k * m)) = 2.0 / (2 * sqrt(100)) = 0.1
        assert_eq!(osc.damping_ratio(2.0), 0.1);

        // Quality factor Q = sqrt(k * m) / R = 10 / 2.0 = 5.0
        assert_eq!(osc.quality_factor(2.0), 5.0);

        // Electrical LC dual
        let lc = HarmonicOscillatorDual::electrical(0.01, 1000.0, 0.001, 0.0);
        assert_eq!(lc.domain(), PhysicalDomain::Electrical);
        assert!((lc.natural_frequency() - (1000.0 / 0.01f64).sqrt()).abs() < 1e-9);

        // Hydraulic dual
        let hyd = HarmonicOscillatorDual::hydraulic(500.0, 20_000.0, 0.05, 1.0);
        assert_eq!(hyd.domain(), PhysicalDomain::Hydraulic);
    }

    #[test]
    fn test_symplectic_error_handling() {
        let mut osc = HarmonicOscillatorDual::translational(1.0, 10.0, 1.0, 0.0);

        // Invalid dt <= 0.0
        assert_eq!(
            osc.step_symplectic(0.0),
            Err(ComputeError::InvalidTimeStep)
        );
        assert_eq!(
            osc.step_symplectic(-0.01),
            Err(ComputeError::InvalidTimeStep)
        );
        assert_eq!(
            osc.step_symplectic(f64::NAN),
            Err(ComputeError::InvalidTimeStep)
        );

        // Invalid parameters
        let mut bad_mass = HarmonicOscillatorDual::translational(0.0, 10.0, 1.0, 0.0);
        assert_eq!(
            bad_mass.step_symplectic(0.01),
            Err(ComputeError::InvalidParameter)
        );

        let mut bad_stiffness = HarmonicOscillatorDual::translational(1.0, -5.0, 1.0, 0.0);
        assert_eq!(
            bad_stiffness.step_symplectic(0.01),
            Err(ComputeError::InvalidParameter)
        );
    }
}
