//! crates/omni/src/galactic_roaming.rs
//! 3D Galactic Node Roaming, 6-DOF Camera Navigation, and Synapse Pulse Particle System.
//!
//! Enables fluid spatial roaming across neural constellation nodes and `.si` cartridge stars.

use serde::{Deserialize, Serialize};

/// 3D Vector for Spatial Coordinates and Velocities
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn distance(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// 6-DOF Galactic Roaming Camera Controller (SPACE-02)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalacticRoamingCameraController {
    pub position: Vec3,
    pub target: Vec3,
    pub pitch: f32, // Pitch angle in radians
    pub yaw: f32,   // Yaw angle in radians
    pub zoom_distance: f32,
    pub smooth_damping: f32,
}

impl Default for GalacticRoamingCameraController {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 100.0, 300.0),
            target: Vec3::ZERO,
            pitch: 0.2,
            yaw: 0.0,
            zoom_distance: 300.0,
            smooth_damping: 0.85,
        }
    }
}

impl GalacticRoamingCameraController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rotates the camera orbit around target based on mouse drag deltas
    pub fn orbit_rotate(&mut self, delta_pitch: f32, delta_yaw: f32) {
        self.pitch = (self.pitch + delta_pitch).clamp(-1.5, 1.5);
        self.yaw += delta_yaw;
        self.update_position();
    }

    /// Zooms camera in or out along the target line
    pub fn zoom(&mut self, factor: f32) {
        self.zoom_distance = (self.zoom_distance * factor).clamp(10.0, 5000.0);
        self.update_position();
    }

    /// Recalculates eye position in spherical coordinates relative to target
    pub fn update_position(&mut self) {
        let cos_p = self.pitch.cos();
        let sin_p = self.pitch.sin();
        let cos_y = self.yaw.cos();
        let sin_y = self.yaw.sin();

        self.position.x = self.target.x + self.zoom_distance * cos_p * sin_y;
        self.position.y = self.target.y + self.zoom_distance * sin_p;
        self.position.z = self.target.z + self.zoom_distance * cos_p * cos_y;
    }

    /// Focuses the camera directly on a specific star node coordinate
    pub fn hyper_jump_to_node(&mut self, node_coord: Vec3) {
        self.target = node_coord;
        self.update_position();
    }
}

/// An individual telemetry pulse particle flowing between neural nodes (SPACE-03)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynapsePulseParticle {
    pub source: Vec3,
    pub destination: Vec3,
    pub current_position: Vec3,
    pub progress: f32, // 0.0 to 1.0
    pub speed: f32,
    pub intensity: f32,
}

/// Synapse Pulse Particle System depicting high-speed telemetry vectors
#[derive(Default)]
pub struct SynapsePulseParticleSystem {
    particles: Vec<SynapsePulseParticle>,
}

impl SynapsePulseParticleSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Spawns a telemetry pulse flowing from source node to destination node
    pub fn spawn_pulse(&mut self, source: Vec3, destination: Vec3, speed: f32) {
        self.particles.push(SynapsePulseParticle {
            source,
            destination,
            current_position: source,
            progress: 0.0,
            speed: speed.clamp(0.01, 1.0),
            intensity: 1.0,
        });
    }

    /// Updates simulation physics for all active particles across delta time
    pub fn tick(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.particles.len() {
            self.particles[i].progress += self.particles[i].speed * dt;
            if self.particles[i].progress >= 1.0 {
                self.particles.swap_remove(i);
            } else {
                let p = self.particles[i].progress;
                let src = self.particles[i].source;
                let dst = self.particles[i].destination;
                self.particles[i].current_position = Vec3::new(
                    src.x + (dst.x - src.x) * p,
                    src.y + (dst.y - src.y) * p,
                    src.z + (dst.z - src.z) * p,
                );
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_orbit_and_hyper_jump() {
        let mut cam = GalacticRoamingCameraController::new();
        let initial_pos = cam.position;

        cam.orbit_rotate(0.1, 0.2);
        assert_ne!(cam.position, initial_pos);

        let target_star = Vec3::new(500.0, 200.0, -100.0);
        cam.hyper_jump_to_node(target_star);
        assert_eq!(cam.target, target_star);
    }

    #[test]
    fn test_synapse_pulse_particle_lifecycle() {
        let mut system = SynapsePulseParticleSystem::new();
        let src = Vec3::ZERO;
        let dst = Vec3::new(100.0, 0.0, 0.0);

        system.spawn_pulse(src, dst, 0.5); // speed 0.5 per sec
        assert_eq!(system.active_particle_count(), 1);

        // Advance 1 sec (progress = 0.5)
        system.tick(1.0);
        assert_eq!(system.active_particle_count(), 1);

        // Advance 1.5 sec (progress = 0.5 + 0.75 = 1.25 -> despawned)
        system.tick(1.5);
        assert_eq!(system.active_particle_count(), 0);
    }
}
