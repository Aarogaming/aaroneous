# Scale-Invariant Dynamics & Physics Compilation Specification

> **CANONICAL SPECIFICATION**  
> **SCOPE**: Bond-Graph Duality, Symplectic Hamiltonian Integration, Analytical Fast-Forwarding, and State Freezing.  
> **APPLIES TO**: `crates/compute`, `crates/capabilities`, `core/hypervisor`.  
> **LAST UPDATED**: 2026-09-12

---

## 1. Executive Summary

[`crates/compute`](../../crates/compute) serves as the numerical execution core of the Aaroneous framework. Beyond neural inference via State-Space Models (SSMs), it incorporates a rigorous multi-domain **Physics Compiler**. The engine models physical environments, robotic actuators, power buses, and thermal sinks through unified **Bond-Graph Duality**, simulating continuous dynamical systems with **Symplectic Hamiltonian Integration** that guarantees phase-space conservation and zero long-term energy drift.

---

## 2. Bond-Graph Duality & Multi-Domain Isomorphisms

Complex cyber-physical systems span multiple physical domains (electrical circuits, electric motors, hydraulic pumps, mechanical linkages, thermal sinks). Rather than writing ad-hoc differential equations for each domain, the Physics Compiler maps all interactions to generalized **Effort ($e$)** and **Flow ($f$)** conjugate power variables:

$$P(t) = e(t) \cdot f(t)$$

Every bond carries power between ports with bidirectional causality.

### 2.1 Domain Isomorphism Table

| Physical Domain | Effort Variable $e(t)$ | Flow Variable $f(t)$ | Generalized Momentum $p = \int e\,dt$ | Generalized Displacement $q = \int f\,dt$ |
|---|---|---|---|---|
| **Mechanical (Translational)** | Force $F\ [\text{N}]$ | Velocity $v\ [\text{m/s}]$ | Linear Momentum $p\ [\text{kg}\cdot\text{m/s}]$ | Position $x\ [\text{m}]$ |
| **Mechanical (Rotational)** | Torque $\tau\ [\text{N}\cdot\text{m}]$ | Angular Velocity $\omega\ [\text{rad/s}]$ | Angular Momentum $L\ [\text{J}\cdot\text{s}]$ | Angle $\theta\ [\text{rad}]$ |
| **Electrical** | Voltage $V\ [\text{V}]$ | Current $i\ [\text{A}]$ | Magnetic Flux Linkage $\lambda\ [\text{Wb}]$ | Electric Charge $q\ [\text{C}]$ |
| **Hydraulic / Fluid** | Pressure $P\ [\text{Pa}]$ | Volumetric Flow $Q\ [\text{m}^3/\text{s}]$ | Pressure Momentum $\Gamma\ [\text{Pa}\cdot\text{s}]$ | Volume $V\ [\text{m}^3]$ |
| **Thermal** | Temperature $T\ [\text{K}]$ | Heat Flow Rate $\dot{Q}\ [\text{W}]$ | — | Thermal Entropy $S\ [\text{J/K}]$ |

### 2.2 Canonical Multi-Port Junctions & Elements

1. **Storage Elements**:
   - **$I$-Element (Inertia / Inductance)**: Stores kinetic/magnetic energy. Constitutive relation: $f = \frac{1}{I} p$.
   - **$C$-Element (Capacitance / Compliance)**: Stores potential/electrostatic energy. Constitutive relation: $e = \frac{1}{C} q$.
2. **Dissipative Elements**:
   - **$R$-Element (Resistance / Friction)**: Dissipates power into heat. Constitutive relation: $e = R \cdot f$.
3. **Power Conserving Junctions**:
   - **0-Junction (Common Effort)**: $e_1 = e_2 = \dots = e_n$ and $\sum_{k=1}^n f_k = 0$ (e.g., Kirchhoff's Current Law, parallel mechanical connections).
   - **1-Junction (Common Flow)**: $f_1 = f_2 = \dots = f_n$ and $\sum_{k=1}^n e_k = 0$ (e.g., Kirchhoff's Voltage Law, series mechanical connections).

---

## 3. Symplectic Hamiltonian Numerical Integration

Standard explicit numerical integrators (e.g., Forward Euler, standard Runge-Kutta RK4) introduce artificial numerical dissipation or non-physical energy growth over long integration epochs. In high-fidelity cyber-physical simulation and real-time HIL digital twins, this causes destabilization or artificial damping.

### 3.1 Hamiltonian Formulation

Systems compiled by `crates/compute` are formulated in canonical Hamiltonian coordinates $(\mathbf{q}, \mathbf{p})$:

$$\mathcal{H}(\mathbf{q}, \mathbf{p}) = T(\mathbf{p}) + V(\mathbf{q})$$

The equations of motion are governed by Hamilton's canonical equations:

$$\dot{\mathbf{q}} = \frac{\partial \mathcal{H}}{\partial \mathbf{p}}, \quad \dot{\mathbf{p}} = -\frac{\partial \mathcal{H}}{\partial \mathbf{q}}$$

### 3.2 Symplectic Structure Preservation

A numerical map $\Phi_{\Delta t}: (\mathbf{q}_t, \mathbf{p}_t) \mapsto (\mathbf{q}_{t+\Delta t}, \mathbf{p}_{t+\Delta t})$ is **symplectic** if its Jacobian matrix $\mathbf{J}$ satisfies:

$$\mathbf{J}^T \mathbf{J}_{\text{sym}} \mathbf{J} = \mathbf{J}_{\text{sym}}, \quad \text{where } \mathbf{J}_{\text{sym}} = \begin{pmatrix} \mathbf{0} & \mathbf{I} \\ -\mathbf{I} & \mathbf{0} \end{pmatrix}$$

By preserving the differential 2-form $\omega = \sum_{k} dq_k \wedge dp_k$, the symplectic Verlet / Leapfrog integrator executed in `crates/compute` ensures:

$$\frac{d\mathcal{H}}{dt} \approx 0 \quad (\text{Bounded energy oscillation with zero secular drift})$$

```
Störmer-Verlet Step:
1. p_{t + 1/2} = p_t - (Δt / 2) · ∇V(q_t)
2. q_{t + 1}   = q_t + Δt · M^{-1} · p_{t + 1/2}
3. p_{t + 1}   = p_{t + 1/2} - (Δt / 2) · ∇V(q_{t + 1})
```

---

## 4. Analytical Fast-Forwarding Across Unobserved Epochs

When physical subsystems are unobserved or operating under quiescent conditions across macroscopic temporal epochs $\Delta T \gg \Delta t_{\text{tick}}$, micro-stepping each individual differential increment wastes hypervisor CPU cycles.

For linear or weakly perturbed dynamical subsystems:

$$\dot{\mathbf{x}}(t) = \mathbf{A} \mathbf{x}(t) + \mathbf{B} \mathbf{u}(t)$$

`crates/compute` computes the exact matrix exponential closed-form solution:

$$\mathbf{x}(t + \Delta T) = \exp(\mathbf{A} \Delta T) \mathbf{x}(t) + \int_{0}^{\Delta T} \exp(\mathbf{A}(\Delta T - \tau)) \mathbf{B} \mathbf{u}(t + \tau)\,d\tau$$

The spectral decomposition $\mathbf{A} = \mathbf{V} \mathbf{\Lambda} \mathbf{V}^{-1}$ is pre-computed at compilation time. Fast-forwarding evaluates in $\mathcal{O}(N)$ diagonal operations rather than thousands of iterative loops, instantly advancing system state to the exact current wall-clock epoch.

---

## 5. Thermodynamic State Freezing at Equilibrium

When an active physical subsystem reaches steady-state equilibrium, its internal entropy generation rate drops below the hypervisor sensitivity threshold $\epsilon_{\text{freeze}}$:

$$\frac{dS_{\text{internal}}}{dt} = \sum_k \frac{P_{\text{dissipated}, k}}{T_k} \le \epsilon_{\text{freeze}} \quad \Longleftrightarrow \quad dG \approx 0$$

### 5.1 Freezing Mechanics
1. **Quiescence Detection**: The compiler monitors $\left\|\dot{\mathbf{x}}\right\|_\infty < \delta_{\text{tol}}$ and $dS/dt < \epsilon_{\text{freeze}}$ over a window of $N_{\text{window}} = 64$ consecutive cycles.
2. **State Freezing**: The dynamical equations are unhooked from the active tick loop. State vectors $\mathbf{x}_{\text{frozen}}$ are placed into read-only shared memory slices (`SwrnRingBuffer`).
3. **Zero Cycle Overhead**: The frozen subsystem consumes exactly $0\,\mu\text{s}$ of CPU time during subsequent hypervisor scans.
4. **Wake-up Interlock**: Any non-zero external input flux ($\|u(t)\| > \delta_{\text{wake}}$) immediately unfreezes the subgraph, restoring full symplectic integration.
