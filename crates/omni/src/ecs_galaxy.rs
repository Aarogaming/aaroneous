//! crates/omni/src/ecs_galaxy.rs
//! High-Performance Entity Component System (ECS) Galaxy World Engine
//! Adapted from Bevy Engine's `bevy_ecs` architecture with `glam` SIMD 3D vector math.
//! Models all code nodes, memory vectors, specialists, and constellation links as decoupled ECS entities.

use bevy_ecs::prelude::*;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Component representing an identifiable Star node in the semantic graph
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct StarNode {
    pub id: String,
    pub name: String,
    pub category: String,
}

/// Component representing 3D spatial position and velocity with SIMD glam::Vec3
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SpatialTransform {
    pub position: Vec3,
    pub velocity: Vec3,
}

impl SpatialTransform {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: Vec3::new(x, y, z),
            velocity: Vec3::ZERO,
        }
    }

    pub fn x(&self) -> f32 {
        self.position.x
    }

    pub fn y(&self) -> f32 {
        self.position.y
    }

    pub fn z(&self) -> f32 {
        self.position.z
    }

    pub fn distance_to(&self, other: &Self) -> f32 {
        self.position.distance(other.position)
    }
}

/// Component holding high-dimensional semantic embedding vectors
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEmbedding {
    pub vector: Vec<f32>,
}

/// Component tracking metabolic token budget of an entity
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct MetabolicEnergy {
    pub current: f32,
    pub max: f32,
}

/// Component tracking constellation links to other star entities
#[derive(Component, Debug, Clone, Default)]
pub struct ConstellationLinks {
    pub connected_entities: Vec<Entity>,
}

/// System that applies orbital velocity and gravitational drift across all stars via SIMD glam
pub fn gravitational_orbit_system(mut query: Query<&mut SpatialTransform>) {
    for mut transform in &mut query {
        let vel = transform.velocity;
        transform.position += vel;
    }
}

/// System that applies metabolic decay/burn rate to active star nodes
pub fn metabolic_decay_system(mut query: Query<&mut MetabolicEnergy>) {
    for mut energy in &mut query {
        energy.current = (energy.current - 0.1).max(0.0);
    }
}

/// High-Performance Bevy ECS Galaxy Engine
pub struct EcsGalaxyEngine {
    pub world: World,
    pub schedule: Schedule,
}

impl Default for EcsGalaxyEngine {
    fn default() -> Self {
        let world = World::new();
        let mut schedule = Schedule::default();

        // Register parallel systems
        schedule.add_systems((gravitational_orbit_system, metabolic_decay_system));

        Self { world, schedule }
    }
}

impl EcsGalaxyEngine {
    /// Spawns a new Star Node Entity into the Bevy ECS World
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_star(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        category: impl Into<String>,
        x: f32,
        y: f32,
        z: f32,
        embedding: Vec<f32>,
    ) -> Entity {
        self.world
            .spawn((
                StarNode {
                    id: id.into(),
                    name: name.into(),
                    category: category.into(),
                },
                SpatialTransform::new(x, y, z),
                SemanticEmbedding { vector: embedding },
                MetabolicEnergy {
                    current: 100.0,
                    max: 100.0,
                },
                ConstellationLinks::default(),
            ))
            .id()
    }

    /// Advances the ECS simulation by one tick across all parallel systems
    pub fn tick(&mut self) {
        self.schedule.run(&mut self.world);
    }

    /// Queries total entity count in the Galaxy World
    pub fn count_stars(&mut self) -> usize {
        let mut query = self.world.query::<&StarNode>();
        query.iter(&self.world).count()
    }

    /// Connects two star entities with a constellation link
    pub fn link_stars(&mut self, star_a: Entity, star_b: Entity) {
        if let Some(mut links_a) = self.world.get_mut::<ConstellationLinks>(star_a) {
            links_a.connected_entities.push(star_b);
        }
        if let Some(mut links_b) = self.world.get_mut::<ConstellationLinks>(star_b) {
            links_b.connected_entities.push(star_a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bevy_ecs_glam_galaxy_spawning_and_simulation() {
        let mut galaxy = EcsGalaxyEngine::default();

        let star_1 = galaxy.spawn_star(
            "star_chimera",
            "Chimera AST",
            "Forge",
            10.0,
            20.0,
            30.0,
            vec![0.1, 0.5, 0.9],
        );

        let star_2 = galaxy.spawn_star(
            "star_synapse",
            "SWMR Synapse",
            "Memory",
            -10.0,
            -20.0,
            -30.0,
            vec![0.9, 0.5, 0.1],
        );

        assert_eq!(galaxy.count_stars(), 2);

        // Test SIMD distance via glam
        let t1 = galaxy.world.get::<SpatialTransform>(star_1).unwrap();
        let t2 = galaxy.world.get::<SpatialTransform>(star_2).unwrap();
        let dist = t1.distance_to(t2);
        assert!(dist > 0.0);

        // Link stars
        galaxy.link_stars(star_1, star_2);
        let links = galaxy.world.get::<ConstellationLinks>(star_1).unwrap();
        assert_eq!(links.connected_entities.len(), 1);
        assert_eq!(links.connected_entities[0], star_2);

        // Set velocity on star_1
        if let Some(mut transform) = galaxy.world.get_mut::<SpatialTransform>(star_1) {
            transform.velocity = Vec3::new(1.0, 2.0, 3.0);
        }

        // Run ECS tick
        galaxy.tick();

        // Verify spatial update
        let transform = galaxy.world.get::<SpatialTransform>(star_1).unwrap();
        assert_eq!(transform.x(), 11.0);
        assert_eq!(transform.y(), 22.0);
        assert_eq!(transform.z(), 33.0);

        // Verify metabolic energy decay
        let energy = galaxy.world.get::<MetabolicEnergy>(star_1).unwrap();
        assert!((energy.current - 99.9).abs() < 1e-4);
    }
}
