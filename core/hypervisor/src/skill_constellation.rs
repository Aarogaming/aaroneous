//! core/hypervisor/src/skill_constellation.rs
//! Skyrim-Style Celestial Constellation Skill Tree & N-Body Latent Physics Engine.
//!
//! Features:
//! 1. Dimensionality mapping: Organic force-directed layout driven by R^256 cosine similarity.
//! 2. Coulomb Repulsion + Hooke's Law Attraction:
//!    Attraction Force = (Current Distance - Target Distance) * Stiffness
//!    Target Distance = Base Distance * (1.0 - Cosine Similarity).
//! 3. Dynamic glowing constellation threads connecting stars with similarity > 0.75.
//! 4. Skyrim-style interactive pan/zoom navigation and detailed lore/telemetry tooltips.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, RichText, Sense, Stroke, Ui, Vec2};
use compute::si_motor_tree::{MotorSkillNode, StarState, MOTOR_INTENT_DIM};

/// A Star Node in the Constellation Visualizer
#[derive(Clone, Debug)]
pub struct VisualStarNode {
    pub id: String,
    pub label: String,
    pub state: StarState,
    pub latent_vector: [f32; MOTOR_INTENT_DIM],
    pub pos: Pos2,
    pub velocity: Vec2,
}

impl VisualStarNode {
    pub fn from_motor_skill(skill: &MotorSkillNode, initial_pos: Pos2) -> Self {
        Self {
            id: skill.id.clone(),
            label: skill.description.clone(),
            state: skill.state.clone(),
            latent_vector: skill.intent_embedding,
            pos: initial_pos,
            velocity: Vec2::ZERO,
        }
    }

    /// Computes R^256 Cosine Similarity
    pub fn cosine_similarity(&self, other: &VisualStarNode) -> f32 {
        let mut dot = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;

        for i in 0..MOTOR_INTENT_DIM {
            dot += self.latent_vector[i] * other.latent_vector[i];
            norm_a += self.latent_vector[i] * self.latent_vector[i];
            norm_b += other.latent_vector[i] * other.latent_vector[i];
        }

        let denom = (norm_a.sqrt() * norm_b.sqrt()).max(1e-6);
        (dot / denom).clamp(-1.0, 1.0)
    }
}

/// The Skyrim-Style Constellation Canvas
pub struct SkillConstellationCanvas {
    pub stars: Vec<VisualStarNode>,
    pub pan_offset: Vec2,
    pub zoom: f32,
    pub is_physics_enabled: bool,
}

impl Default for SkillConstellationCanvas {
    fn default() -> Self {
        let mut stars = Vec::new();

        let mut v1 = [0.0f32; MOTOR_INTENT_DIM];
        v1[0] = 0.9; v1[1] = 0.3;
        stars.push(VisualStarNode {
            id: "BASE_UI_TREE".into(),
            label: "Identify UI Automation Tree".into(),
            state: StarState::Crystallized { addr: 0x7FFA_1111, time_ns: 120 },
            latent_vector: v1,
            pos: Pos2::new(100.0, 100.0),
            velocity: Vec2::ZERO,
        });

        let mut v2 = [0.0f32; MOTOR_INTENT_DIM];
        v2[0] = 0.85; v2[1] = 0.35;
        stars.push(VisualStarNode {
            id: "AUTH_TRAVERSAL".into(),
            label: "Sovereign Auth Traversal".into(),
            state: StarState::Compiling,
            latent_vector: v2,
            pos: Pos2::new(220.0, 80.0),
            velocity: Vec2::ZERO,
        });

        let mut v3 = [0.0f32; MOTOR_INTENT_DIM];
        v3[0] = 0.80; v3[1] = 0.40;
        stars.push(VisualStarNode {
            id: "DATA_INJECTION".into(),
            label: "Direct Memory Stream".into(),
            state: StarState::Neural { variance: 0.12 },
            latent_vector: v3,
            pos: Pos2::new(280.0, 180.0),
            velocity: Vec2::ZERO,
        });

        let mut v4 = [0.0f32; MOTOR_INTENT_DIM];
        v4[50] = 0.95; // Orthogonal cluster
        stars.push(VisualStarNode {
            id: "AST_MUTATE".into(),
            label: "In-Place AST Mutation".into(),
            state: StarState::Crystallized { addr: 0x7FFA_4444, time_ns: 85 },
            latent_vector: v4,
            pos: Pos2::new(-150.0, -100.0),
            velocity: Vec2::ZERO,
        });

        Self {
            stars,
            pan_offset: Vec2::ZERO,
            zoom: 1.0,
            is_physics_enabled: true,
        }
    }
}

impl SkillConstellationCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// Simulate N-body Latent Physics (Coulomb Repulsion + Hooke's Law Attraction)
    pub fn simulate_physics(&mut self) {
        if !self.is_physics_enabled || self.stars.len() < 2 {
            return;
        }

        let delta_time = 0.016f32; // ~60 FPS
        let repulsion_strength = 3000.0f32;
        let spring_stiffness = 6.0f32;
        let damping = 0.88f32;
        let base_distance = 250.0f32;

        let n = self.stars.len();
        let mut forces = vec![Vec2::ZERO; n];

        for i in 0..n {
            for j in (i + 1)..n {
                let dir = self.stars[j].pos - self.stars[i].pos;
                let mut dist = dir.length();
                if dist < 0.1 {
                    dist = 0.1;
                }
                let dir_norm = dir / dist;

                // 1. Coulomb Repulsion
                let repulse = dir_norm * (repulsion_strength / (dist * dist));
                forces[i] -= repulse;
                forces[j] += repulse;

                // 2. Hooke's Law Attraction based on R^256 Cosine Similarity
                let similarity = self.stars[i].cosine_similarity(&self.stars[j]);
                if similarity > 0.2 {
                    let target_dist = base_distance * (1.0 - similarity);
                    let displacement = dist - target_dist;
                    let attraction = dir_norm * (displacement * spring_stiffness);
                    forces[i] += attraction;
                    forces[j] += attraction;
                }
            }
        }

        // Integrate velocity & positions
        for i in 0..n {
            self.stars[i].velocity += forces[i] * delta_time;
            self.stars[i].velocity *= damping;
            let step = self.stars[i].velocity * delta_time;
            self.stars[i].pos += step;
        }
    }

    /// Render Skyrim-style custom constellation painter loop
    pub fn update_ui(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        self.simulate_physics();

        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

        // 1. Handle Pan and Zoom Navigation
        if response.dragged() {
            self.pan_offset += response.drag_delta();
        }
        if response.hovered() {
            let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom *= if scroll > 0.0 { 1.08 } else { 0.92 };
                self.zoom = self.zoom.clamp(0.2, 4.0);
            }
        }

        let center = response.rect.center();
        let zoom = self.zoom;
        let pan = self.pan_offset;

        let to_screen = |pos: Pos2| -> Pos2 {
            let scaled = pos.to_vec2() * zoom;
            center + scaled + pan
        };

        // 2. Paint Deep Space Void Background
        painter.rect_filled(response.rect, 0.0, Color32::from_rgb(10, 13, 20));

        // 3. Dynamically Draw Constellation Lines based on Latent Similarity
        for i in 0..self.stars.len() {
            for j in (i + 1)..self.stars.len() {
                let similarity = self.stars[i].cosine_similarity(&self.stars[j]);
                if similarity > 0.70 {
                    let start_screen = to_screen(self.stars[i].pos);
                    let target_screen = to_screen(self.stars[j].pos);
                    let alpha = (((similarity - 0.70) / 0.30) * 220.0).clamp(30.0, 255.0) as u8;

                    painter.line_segment(
                        [start_screen, target_screen],
                        Stroke::new(2.0 * zoom, Color32::from_rgba_unmultiplied(120, 180, 255, alpha)),
                    );
                }
            }
        }

        // 4. Draw Stars and Interactive Tooltips
        if let Some(hover_pos) = response.hover_pos() {
            for star in &self.stars {
                let screen_pos = to_screen(star.pos);
                let base_radius = 12.0 * zoom;
                let is_hovered = screen_pos.distance(hover_pos) < base_radius * 1.5;

                let (color, radius_mult, has_glow) = match &star.state {
                    StarState::Neural { .. } => (Color32::from_rgb(90, 140, 255), 1.0, false), // Dim Blue
                    StarState::Compiling => (Color32::from_rgb(255, 210, 50), 1.25, true),     // Pulsing Yellow
                    StarState::Crystallized { .. } => (Color32::from_rgb(50, 255, 130), 1.4, true), // Brilliant Gold/Green
                };

                let current_radius = base_radius * radius_mult;

                // Outer Halo Glow
                if has_glow {
                    painter.circle_filled(
                        screen_pos,
                        current_radius * 1.8,
                        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40),
                    );
                }

                // Star Solid Core
                painter.circle_filled(screen_pos, current_radius, color);
                painter.circle_stroke(screen_pos, current_radius, Stroke::new(1.5, Color32::WHITE));

                // Star Label
                painter.text(
                    screen_pos + Vec2::new(0.0, current_radius + 8.0),
                    Align2::CENTER_TOP,
                    &star.id,
                    FontId::proportional(12.0 * zoom),
                    Color32::from_rgb(220, 230, 245),
                );

                // Hover Tooltip
                if is_hovered {
                    egui::Tooltip::always_open(
                        ctx.clone(),
                        ui.layer_id(),
                        ui.id().with(&star.id),
                        egui::containers::PopupAnchor::Pointer,
                    )
                    .show(|ui: &mut Ui| {
                        ui.heading(format!("⭐ {}", star.id));
                        ui.label(&star.label);
                        ui.separator();
                        match &star.state {
                            StarState::Neural { variance } => {
                                ui.colored_label(Color32::LIGHT_BLUE, "Status: Neural Inference (Online Learning)");
                                ui.label(format!("LoRA Gradient Variance: {:.4}", variance));
                            }
                            StarState::Compiling => {
                                ui.colored_label(Color32::YELLOW, "Status: JIT Compiling to Bare-Metal");
                                ui.label("W^X Memory Arena: Flipping RW -> RX");
                            }
                            StarState::Crystallized { addr, time_ns } => {
                                ui.colored_label(Color32::GREEN, "Status: Mastered Reflex (Crystallized)");
                                ui.label(RichText::new(format!("Memory Address: {:#X}", addr)).monospace());
                                ui.label(format!("Execution Latency: {} ns (< 1µs)", time_ns));
                            }
                        }
                    });
                }
            }
        }

        ctx.request_repaint(); // Keep physics running smoothly at 60Hz
    }
}
