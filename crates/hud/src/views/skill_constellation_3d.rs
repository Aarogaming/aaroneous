// core/hypervisor/src/hud/views/skill_constellation_3d.rs
//! 3D Constellation Skill Tree Viewport.
//! Celestial branching progression map for training .si model skills and perks.
//! Uses pure astrophysical and mathematical metaphors (Perception, Kinematics, Reflex, Thermodynamics).

use crate::hud::state::SharedHudState;
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstellationSkillNode {
    pub id: String,
    pub name: String,
    pub cluster: String,
    pub tier: u32,
    pub cost_points: u32,
    pub is_unlocked: bool,
    pub mastery_level: u32,
    pub pos: [f32; 3], // 3D coordinates in constellation space
    pub connections: Vec<String>,
    pub description: String,
}

pub struct SkillConstellation3DView {
    pub selected_node_id: Option<String>,
    pub filter_cluster: String,
    pub nodes: Vec<ConstellationSkillNode>,
}

impl Default for SkillConstellation3DView {
    fn default() -> Self {
        Self {
            selected_node_id: Some("percept_root".to_string()),
            filter_cluster: String::new(),
            nodes: Self::default_nodes(),
        }
    }
}

impl SkillConstellation3DView {
    fn default_nodes() -> Vec<ConstellationSkillNode> {
        vec![
            // Perception Cluster (Cyan)
            ConstellationSkillNode {
                id: "percept_root".to_string(),
                name: "Spatial Gating".to_string(),
                cluster: "Perception".to_string(),
                tier: 1,
                cost_points: 1,
                is_unlocked: true,
                mastery_level: 3,
                pos: [-140.0, 100.0, 0.0],
                connections: vec!["percept_ocr".into(), "percept_delta".into()],
                description: "Filters static pixels instantly with zero compute overhead.".to_string(),
            },
            ConstellationSkillNode {
                id: "percept_ocr".to_string(),
                name: "Text & Symbol Anchor".to_string(),
                cluster: "Perception".to_string(),
                tier: 2,
                cost_points: 2,
                is_unlocked: true,
                mastery_level: 2,
                pos: [-220.0, 160.0, -30.0],
                connections: vec![],
                description: "Anchors onto dynamic UI labels and in-game text triggers.".to_string(),
            },
            ConstellationSkillNode {
                id: "percept_delta".to_string(),
                name: "Motion Vector Sense".to_string(),
                cluster: "Perception".to_string(),
                tier: 2,
                cost_points: 2,
                is_unlocked: false,
                mastery_level: 0,
                pos: [-160.0, 190.0, 40.0],
                connections: vec![],
                description: "Predicts trajectory of moving targets across frame intervals.".to_string(),
            },
            // Kinematics Cluster (Gold)
            ConstellationSkillNode {
                id: "kinematics_root".to_string(),
                name: "Bezier Kinematics".to_string(),
                cluster: "Kinematics".to_string(),
                tier: 1,
                cost_points: 1,
                is_unlocked: true,
                mastery_level: 4,
                pos: [140.0, 100.0, 0.0],
                connections: vec!["kinematics_fitts".into(), "kinematics_jitter".into()],
                description: "Human-like natural curvature with realistic acceleration curves.".to_string(),
            },
            ConstellationSkillNode {
                id: "kinematics_fitts".to_string(),
                name: "Fitts-Law Optimization".to_string(),
                cluster: "Kinematics".to_string(),
                tier: 2,
                cost_points: 2,
                is_unlocked: true,
                mastery_level: 1,
                pos: [220.0, 160.0, -20.0],
                connections: vec![],
                description: "Calibrates travel velocity according to target bounding area.".to_string(),
            },
            ConstellationSkillNode {
                id: "kinematics_jitter".to_string(),
                name: "Micro-Tremor Emulation".to_string(),
                cluster: "Kinematics".to_string(),
                tier: 2,
                cost_points: 2,
                is_unlocked: false,
                mastery_level: 0,
                pos: [170.0, 190.0, 50.0],
                connections: vec![],
                description: "Injects physiological sub-pixel jitter to prevent anti-bot detection.".to_string(),
            },
            // Reflex Cluster (Purple)
            ConstellationSkillNode {
                id: "reflex_root".to_string(),
                name: "Instant Dispatch".to_string(),
                cluster: "Reflex".to_string(),
                tier: 1,
                cost_points: 1,
                is_unlocked: true,
                mastery_level: 2,
                pos: [-60.0, -80.0, 20.0],
                connections: vec!["reflex_branching".into()],
                description: "Dispatches keyboard and mouse actions with sub-180µs latency.".to_string(),
            },
            ConstellationSkillNode {
                id: "reflex_branching".to_string(),
                name: "Adaptive Fallback".to_string(),
                cluster: "Reflex".to_string(),
                tier: 2,
                cost_points: 3,
                is_unlocked: false,
                mastery_level: 0,
                pos: [-100.0, -160.0, -30.0],
                connections: vec![],
                description: "Instantly routes to secondary fallback paths upon sensory interruption.".to_string(),
            },
            // Thermodynamics Cluster (Emerald)
            ConstellationSkillNode {
                id: "thermo_root".to_string(),
                name: "Zero Waste Slab".to_string(),
                cluster: "Thermodynamics".to_string(),
                tier: 1,
                cost_points: 1,
                is_unlocked: true,
                mastery_level: 3,
                pos: [80.0, -70.0, -20.0],
                connections: vec!["thermo_compact".into()],
                description: "Maintains minimal thermodynamic free energy dissipation.".to_string(),
            },
            ConstellationSkillNode {
                id: "thermo_compact".to_string(),
                name: "Dense Memory Pack".to_string(),
                cluster: "Thermodynamics".to_string(),
                tier: 2,
                cost_points: 3,
                is_unlocked: false,
                mastery_level: 0,
                pos: [140.0, -140.0, 40.0],
                connections: vec![],
                description: "Compacts dormant state weights into zero-copy SIMD memory banks.".to_string(),
            },
        ]
    }
}

impl HudView for SkillConstellation3DView {
    fn id(&self) -> &'static str {
        "skill_constellation_3d"
    }

    fn title(&self) -> &'static str {
        "🌌 Constellation Skills"
    }

    fn render(&mut self, ui: &mut egui::Ui, state: &mut SharedHudState) {
        let theme = state.settings.theme;
        let time_sec = state.start_time.elapsed().as_secs_f32();

        // 1. Header & Mastery Currency Bar
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("🌌 3D Model Skill Constellation")
                    .color(theme.accent())
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Reset View").clicked() {
                    state.camera_pan = Vec2::ZERO;
                    state.camera_zoom = 1.0;
                    state.camera_rotation = (0.3, 0.2);
                }
                ui.label(
                    egui::RichText::new(format!("⭐ Available Mastery: {} pts", state.user_level * 2))
                        .color(Color32::from_rgb(255, 215, 0))
                        .strong(),
                );
            });
        });

        ui.label(
            "Spatial Progression Architecture: Unlock and master capabilities across Perception, Kinematics, Reflex, and Thermodynamic clusters.",
        );
        ui.add_space(4.0);

        // 2. Cluster Filter Selector
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Branch Focus:").strong());
            for cluster in ["All", "Perception", "Kinematics", "Reflex", "Thermodynamics"] {
                let is_sel = if self.filter_cluster.is_empty() {
                    cluster == "All"
                } else {
                    self.filter_cluster == cluster
                };
                if ui.selectable_label(is_sel, cluster).clicked() {
                    self.filter_cluster = if cluster == "All" {
                        String::new()
                    } else {
                        cluster.to_string()
                    };
                }
            }
        });
        ui.separator();

        // 3. Allocate 3D Canvas
        let canvas_height = 420.0;
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), canvas_height),
            egui::Sense::click_and_drag(),
        );

        let canvas_rect = response.rect;
        let center = canvas_rect.center() + state.camera_pan;

        // Camera Drag & Zoom Controls
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            state.camera_rotation.0 += delta.x * 0.008; // Yaw
            state.camera_rotation.1 = (state.camera_rotation.1 + delta.y * 0.008).clamp(-1.4, 1.4); // Pitch
        } else if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            state.camera_pan += response.drag_delta();
        }

        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if response.hovered() && scroll_delta.abs() > 0.1 {
            let factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
            state.camera_zoom = (state.camera_zoom * factor).clamp(0.5, 3.5);
        }

        // Draw Canvas Backdrop
        painter.rect_filled(canvas_rect, CornerRadius::same(8), Color32::from_rgb(8, 12, 20));
        painter.rect_stroke(
            canvas_rect,
            CornerRadius::same(8),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(60, 80, 120, 100)),
            egui::StrokeKind::Inside,
        );

        // 3D Perspective Projection Function
        let yaw = state.camera_rotation.0;
        let pitch: f32 = state.camera_rotation.1;
        let zoom = state.camera_zoom;

        let project_3d = |pos: [f32; 3]| -> Option<(Pos2, f32, f32)> {
            let x = pos[0];
            let y = pos[1];
            let z = pos[2];

            // 1. Yaw rotation (around Y axis)
            let cos_y = yaw.cos();
            let sin_y = yaw.sin();
            let x1 = x * cos_y + z * sin_y;
            let z1 = -x * sin_y + z * cos_y;

            // 2. Pitch rotation (around X axis)
            let cos_p = pitch.cos();
            let sin_p = pitch.sin();
            let y2 = y * cos_p - z1 * sin_p;
            let z2 = y * sin_p + z1 * cos_p;

            // Camera distance offset
            let cam_dist = 500.0;
            let z_cam = z2 + cam_dist;
            if z_cam <= 20.0 {
                return None;
            }

            let fov = 450.0 * zoom;
            let screen_x = center.x + (x1 * fov) / z_cam;
            let screen_y = center.y - (y2 * fov) / z_cam;
            let scale = fov / z_cam;

            Some((Pos2::new(screen_x, screen_y), scale, z_cam))
        };

        // Render Ambient Background Star Field
        for i in 0..70 {
            let seed = (i as f32) * 123.456;
            let sx = (seed * 17.3).sin() * 400.0;
            let sy = (seed * 29.7).cos() * 250.0;
            let sz = (seed * 43.1).sin() * 300.0;

            if let Some((pt, scale, z_cam)) = project_3d([sx, sy, sz]) {
                if canvas_rect.contains(pt) {
                    let tw = ((time_sec * 2.5 + i as f32 * 1.5).sin() * 0.5 + 0.5).clamp(0.2, 1.0);
                    let alpha = ((140.0 * tw) * (1.0 - (z_cam / 1100.0).clamp(0.0, 0.8))) as u8;
                    painter.circle_filled(
                        pt,
                        (1.2 * scale).clamp(0.6, 2.4),
                        Color32::from_rgba_unmultiplied(190, 210, 255, alpha),
                    );
                }
            }
        }

        // Build position cache & lookup map
        let mut node_positions: HashMap<String, (Pos2, f32)> = HashMap::new();
        let mut sorted_nodes: Vec<&ConstellationSkillNode> = self
            .nodes
            .iter()
            .filter(|n| self.filter_cluster.is_empty() || n.cluster == self.filter_cluster)
            .collect();

        // Sort by Z for proper 3D depth rendering (furthest first)
        sorted_nodes.sort_by(|a, b| {
            let za = a.pos[2];
            let zb = b.pos[2];
            za.partial_cmp(&zb).unwrap_or(std::cmp::Ordering::Equal)
        });

        for node in &self.nodes {
            if let Some((pos_2d, scale, _)) = project_3d(node.pos) {
                node_positions.insert(node.id.clone(), (pos_2d, scale));
            }
        }

        let node_map: HashMap<String, ConstellationSkillNode> =
            self.nodes.iter().map(|n| (n.id.clone(), n.clone())).collect();

        // Draw Constellation Energy Wires (Connections)
        for node in &self.nodes {
            if let Some(&(p1, scale1)) = node_positions.get(&node.id) {
                for conn_id in &node.connections {
                    if let Some(target) = node_map.get(conn_id) {
                        if let Some((p2, scale2, _)) = project_3d(target.pos) {
                            let wire_color = if node.is_unlocked && target.is_unlocked {
                                Color32::from_rgba_unmultiplied(120, 190, 255, 180)
                            } else {
                                Color32::from_rgba_unmultiplied(70, 85, 110, 90)
                            };

                            let wire_width = if node.is_unlocked && target.is_unlocked {
                                (2.5 * scale1.min(scale2)).clamp(1.5, 4.0)
                            } else {
                                (1.0 * scale1.min(scale2)).clamp(0.8, 2.0)
                            };

                            painter.line_segment([p1, p2], Stroke::new(wire_width, wire_color));

                            // Animated energy pulse traveling along active connections
                            if node.is_unlocked && target.is_unlocked {
                                let t_pulse = (time_sec * 1.5 + (node.tier as f32) * 0.4) % 1.0;
                                let pulse_pos = Pos2::new(
                                    p1.x + (p2.x - p1.x) * t_pulse,
                                    p1.y + (p2.y - p1.y) * t_pulse,
                                );
                                painter.circle_filled(
                                    pulse_pos,
                                    (2.5 * scale1).clamp(1.5, 4.5),
                                    Color32::from_rgb(255, 255, 255),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Draw Constellation Skill Nodes
        let mut clicked_id: Option<String> = None;
        let mouse_pos = ui.input(|i| i.pointer.hover_pos());

        for node in &sorted_nodes {
            if let Some(&(pos_2d, scale)) = node_positions.get(&node.id) {
                let radius = (12.0 * scale).clamp(8.0, 28.0);
                let is_selected = self.selected_node_id.as_deref() == Some(node.id.as_str());
                let is_hovered = mouse_pos.map_or(false, |mp| mp.distance(pos_2d) <= radius + 4.0);

                if is_hovered && ui.input(|i| i.pointer.primary_clicked()) {
                    clicked_id = Some(node.id.clone());
                }

                // Determine Node Cluster Color
                let base_color = match node.cluster.as_str() {
                    "Perception" => Color32::from_rgb(56, 189, 248),   // Cyan
                    "Kinematics" => Color32::from_rgb(251, 191, 36),   // Amber/Gold
                    "Reflex" => Color32::from_rgb(168, 85, 247),       // Purple
                    "Thermodynamics" => Color32::from_rgb(52, 211, 153),// Emerald
                    _ => Color32::from_rgb(148, 163, 184),
                };

                let node_color = if node.is_unlocked {
                    base_color
                } else {
                    Color32::from_rgb(60, 70, 85)
                };

                // Outer pulsing glow if unlocked
                if node.is_unlocked {
                    let (r, g, b) = (base_color.r(), base_color.g(), base_color.b());
                    let fog = (1.0 - (node.pos[2] / 800.0).clamp(0.0, 0.7)).max(0.2);
                    let pulse = ((time_sec * 2.5).sin() * 0.5 + 0.5) * 0.3 + 0.7;
                    painter.circle_filled(
                        pos_2d,
                        radius * 1.8 * pulse,
                        Color32::from_rgba_unmultiplied(r, g, b, (40.0 * fog) as u8),
                    );
                }

                // Core circle
                painter.circle_filled(pos_2d, radius, node_color);
                painter.circle_stroke(
                    pos_2d,
                    radius,
                    Stroke::new(1.5, if node.is_unlocked { Color32::WHITE } else { Color32::GRAY }),
                );

                // Selection ring
                if is_selected {
                    painter.circle_stroke(
                        pos_2d,
                        radius + 6.0 * scale,
                        Stroke::new(2.0, Color32::from_rgb(255, 255, 255)),
                    );
                }

                // Node Name Label
                let font_size = (11.0 * scale).clamp(9.0, 14.0);
                painter.text(
                    pos_2d + Vec2::new(radius + 4.0, -font_size * 0.5),
                    egui::Align2::LEFT_CENTER,
                    &node.name,
                    egui::FontId::proportional(font_size),
                    if node.is_unlocked { Color32::WHITE } else { Color32::GRAY },
                );
            }
        }

        if let Some(id) = clicked_id {
            self.selected_node_id = Some(id);
        }

        ui.add_space(8.0);

        // 4. Selected Perk Inspector Panel
        let mut toggle_unlock_id: Option<String> = None;
        let mut upgrade_mastery_id: Option<String> = None;

        if let Some(node) = self
            .selected_node_id
            .as_ref()
            .and_then(|id| node_map.get(id))
        {
            egui::Frame::group(ui.style())
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0, theme.accent()))
                .corner_radius(CornerRadius::same(8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new(format!("✦ {}", node.name))
                                .color(theme.accent())
                                .strong(),
                        );
                        ui.label(format!("Branch: {}", node.cluster));
                        ui.label(format!("Tier {}", node.tier));

                        if node.is_unlocked {
                            ui.label(
                                egui::RichText::new(format!("Mastery: Lv. {}", node.mastery_level))
                                    .color(Color32::from_rgb(255, 215, 0))
                                    .strong(),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new(format!("Cost: {} Mastery Pts", node.cost_points))
                                    .color(Color32::from_rgb(255, 165, 0))
                                    .strong(),
                            );
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !node.is_unlocked {
                                if ui.button(egui::RichText::new("⚡ Unlock Perk").strong().color(Color32::WHITE)).clicked() {
                                    toggle_unlock_id = Some(node.id.clone());
                                }
                            } else if ui.button("⚡ Upgrade Mastery (+1)").clicked() {
                                upgrade_mastery_id = Some(node.id.clone());
                            }
                        });
                    });

                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&node.description).italics());
                });
        }

        // Apply stateful unlocks and mastery upgrades
        if let Some(id) = toggle_unlock_id {
            if let Some(target_node) = self.nodes.iter_mut().find(|n| n.id == id) {
                target_node.is_unlocked = true;
                target_node.mastery_level = 1;
                state.award_xp(150, "Unlocked Constellation Skill Perk");
                state.trigger_achievement_progress("constellation_initiate", 1);
            }
        }
        if let Some(id) = upgrade_mastery_id {
            if let Some(target_node) = self.nodes.iter_mut().find(|n| n.id == id) {
                target_node.mastery_level += 1;
                state.award_xp(75, "Upgraded Skill Mastery");
                state.trigger_achievement_progress("mastery_ascendant", 1);
            }
        }
    }
}
