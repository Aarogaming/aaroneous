// core/hypervisor/src/hud/views/galaxy_map_3d.rs
//! 3D Constellation Studio & Spatial Knowledge Graph Viewport (Phase 17 Sovereign 3D Cosmos).

use crate::hud::state::{GalaxyStar, SharedHudState};
use crate::hud::views::HudView;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Stroke, Vec2};
use std::collections::HashMap;

#[derive(Default)]
pub struct Galaxy3DView;

impl HudView for Galaxy3DView {
    fn id(&self) -> &'static str {
        "galaxy_map_3d"
    }

    fn title(&self) -> &'static str {
        "🌌 3D Galaxy"
    }

    fn render(&mut self, ui: &mut egui::Ui, state: &mut SharedHudState) {
        let theme = state.settings.theme;
        let time_sec = state.start_time.elapsed().as_secs_f32();

        // 1. Header & Controls
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("🌌 3D Constellation Studio & Spatial Knowledge Graph")
                    .color(theme.accent())
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Reset Camera").clicked() {
                    state.camera_pan = Vec2::ZERO;
                    state.camera_zoom = 1.0;
                    state.camera_rotation = (0.4, 0.3);
                }
                ui.toggle_value(&mut state.galaxy_auto_rotate, "💫 Auto-Orbit");
            });
        });

        ui.label(
            "Pure-Rust 3D Spatial Hypervisor: Drag canvas to rotate camera (360°), Scroll to zoom, Click stars to inspect live specialist substrates.",
        );
        ui.add_space(4.0);

        // 2. Category Filter Chips
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Filter Cluster:").strong());
            for cat in [
                "All",
                "Specialists",
                "Reflex",
                "Memory",
                "Security",
                "Networking",
                "Capture",
            ] {
                let is_selected = state.galaxy_filter_category == cat;
                if ui.selectable_label(is_selected, cat).clicked() {
                    state.galaxy_filter_category = cat.to_string();
                }
            }
        });
        ui.separator();

        // Auto-rotation
        if state.galaxy_auto_rotate {
            state.camera_rotation.0 += 0.003;
            ui.ctx().request_repaint();
        }

        // 3. 3D Canvas Viewport Allocation
        let canvas_height = 420.0;
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), canvas_height),
            egui::Sense::click_and_drag(),
        );

        let canvas_rect = response.rect;
        let center = canvas_rect.center() + state.camera_pan;

        // Mouse Drag Orbit & Zoom Controls (Natural pitch & yaw orientation)
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            state.camera_rotation.0 += delta.x * 0.008; // Yaw
            state.camera_rotation.1 = (state.camera_rotation.1 + delta.y * 0.008).clamp(-1.4, 1.4); // Natural Pitch
        } else if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            state.camera_pan += response.drag_delta();
        }

        // Scroll Zoom
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if response.hovered() && scroll_delta.abs() > 0.1 {
            let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
            state.camera_zoom = (state.camera_zoom * zoom_factor).clamp(0.4, 3.5);
        }

        // Background Cosmos Backdrop
        painter.rect_filled(
            canvas_rect,
            CornerRadius::same(6),
            Color32::from_rgb(8, 10, 16),
        );

        // Draw 3D Concentric Orbital Rings on Ground Plane (XZ)
        let (yaw, pitch) = state.camera_rotation;
        let cos_y = yaw.cos();
        let sin_y = yaw.sin();
        let cos_p = pitch.cos();
        let sin_p = pitch.sin();
        let focal_dist = 500.0;
        let cam_dist = 450.0 / state.camera_zoom;

        // 3D Projection Closure
        let project_3d = |pos: [f32; 3]| -> Option<(Pos2, f32, f32)> {
            let x = pos[0];
            let y = pos[1];
            let z = pos[2];

            // 1. Yaw rotation (Y-axis)
            let x1 = x * cos_y - z * sin_y;
            let z1 = x * sin_y + z * cos_y;

            // 2. Pitch rotation (X-axis)
            let y2 = y * cos_p - z1 * sin_p;
            let z2 = y * sin_p + z1 * cos_p;

            // 3. Camera distance & perspective projection
            let z_cam = z2 + cam_dist;
            if z_cam <= 10.0 {
                return None; // Behind camera plane
            }

            let scale = focal_dist / z_cam;
            let screen_x = center.x + x1 * scale;
            let screen_y = center.y + y2 * scale;

            Some((Pos2::new(screen_x, screen_y), scale, z_cam))
        };

        // Render Deep Parallax Starfield (Magical game-like backdrop)
        for i in 0..120 {
            // Deterministic pseudo-random 3D positions spread in a sphere
            let seed = (i as f32) * 12.9898;
            let star_x = (seed.sin() * 800.0) % 600.0;
            let star_y = ((seed * 1.5).cos() * 600.0) % 400.0;
            let star_z = ((seed * 2.3).sin() * 800.0) % 600.0;

            if let Some((pos_2d, scale, z_cam)) = project_3d([star_x, star_y, star_z]) {
                if canvas_rect.contains(pos_2d) {
                    let twinkle = ((time_sec * 3.0 + i as f32 * 1.7).sin() * 0.5 + 0.5).clamp(0.2, 1.0);
                    let star_alpha = ((180.0 * twinkle) * (1.0 - (z_cam / 1200.0).clamp(0.0, 0.8))) as u8;
                    let star_radius = (1.5 * scale).clamp(0.8, 2.5);
                    
                    // Subtle tint variations (blue-white, gold, purple)
                    let tint = match i % 4 {
                        0 => Color32::from_rgba_unmultiplied(180, 220, 255, star_alpha),
                        1 => Color32::from_rgba_unmultiplied(255, 235, 180, star_alpha),
                        2 => Color32::from_rgba_unmultiplied(210, 180, 255, star_alpha),
                        _ => Color32::from_rgba_unmultiplied(255, 255, 255, star_alpha),
                    };
                    painter.circle_filled(pos_2d, star_radius, tint);
                }
            }
        }

        // Draw Orbital Rings (XZ plane)
        for ring_radius in [80.0f32, 160.0, 240.0] {
            let segments = 48;
            let mut ring_pts = Vec::new();
            for seg in 0..=segments {
                let theta = (seg as f32 / segments as f32) * std::f32::consts::TAU;
                let rx = ring_radius * theta.cos();
                let rz = ring_radius * theta.sin();
                if let Some((pt, _, _)) = project_3d([rx, 0.0, rz]) {
                    ring_pts.push(pt);
                }
            }
            for w in ring_pts.windows(2) {
                painter.line_segment(
                    [w[0], w[1]],
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(40, 60, 95, 50)),
                );
            }
        }

        // 4. Filter & Collect Projected Nodes
        let filter = state.galaxy_filter_category.clone();
        let star_map: HashMap<String, GalaxyStar> = state
            .galaxy_stars
            .iter()
            .map(|s| (s.id.clone(), s.clone()))
            .collect();

        // 5. Draw 3D Constellation Edges & Dynamic Traveling Pulses
        let mut drawn_edges = std::collections::HashSet::new();
        for star in &state.galaxy_stars {
            if filter != "All" && star.category != filter {
                continue;
            }

            if let Some((p1, scale1, _)) = project_3d(star.pos) {
                for target_id in &star.connected_to {
                    let edge_key = if star.id < *target_id {
                        format!("{}:{}", star.id, target_id)
                    } else {
                        format!("{}:{}", target_id, star.id)
                    };

                    if drawn_edges.insert(edge_key)
                        && let Some((p2, scale2, _)) = star_map
                            .get(target_id)
                            .and_then(|target_star| project_3d(target_star.pos))
                    {
                        // Constellation Wire
                        let alpha = ((scale1 + scale2) * 45.0).clamp(20.0, 140.0) as u8;
                        painter.line_segment(
                            [p1, p2],
                            Stroke::new(1.2, Color32::from_rgba_unmultiplied(56, 139, 253, alpha)),
                        );

                        // Animated Real-Time Execution Pulse (Driven by live bus activity)
                        let bus_speed_multiplier = (state.bus_events_per_sec / 1000.0).clamp(0.5, 4.0);
                        let star_activity = star.activity_level.clamp(0.1, 1.0);
                        let pulse_phase = (time_sec * (0.6 * bus_speed_multiplier * star_activity)
                            + (star.domain_opcode as f32 * 0.05))
                            % 1.0;
                        let pulse_pos = Pos2::new(
                            p1.x + (p2.x - p1.x) * pulse_phase,
                            p1.y + (p2.y - p1.y) * pulse_phase,
                        );
                        let pulse_scale = (scale1 + (scale2 - scale1) * pulse_phase).max(0.5);

                        // Glow color shifts with live activity: active white/cyan when bus is active
                        let pulse_color = if state.is_live_bus {
                            Color32::from_rgba_unmultiplied(230, 250, 255, 240)
                        } else {
                            Color32::from_rgba_unmultiplied(180, 200, 230, 160)
                        };

                        painter.circle_filled(pulse_pos, 3.5 * pulse_scale, pulse_color);
                        painter.circle_stroke(
                            pulse_pos,
                            6.5 * pulse_scale,
                            Stroke::new(1.0, Color32::from_rgba_unmultiplied(56, 139, 253, 180)),
                        );
                    }
                }
            }
        }

        // 6. Draw 3D Star Nodes (Z-Sorted from Back to Front)
        let mut projected_stars = Vec::new();
        for star in &state.galaxy_stars {
            if filter != "All" && star.category != filter {
                continue;
            }
            if let Some((pos_2d, scale, z_cam)) = project_3d(star.pos) {
                projected_stars.push((star, pos_2d, scale, z_cam));
            }
        }

        // Sort descending by z_cam (draw furthest first)
        projected_stars.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        let click_pos = if response.clicked() {
            response.interact_pointer_pos()
        } else {
            None
        };
        let mut clicked_star_id = None;

        for (star, pos_2d, scale, z_cam) in &projected_stars {
            let is_selected = state.selected_galaxy_star_id.as_deref() == Some(star.id.as_str());
            let base_radius = if is_selected { 14.0 } else { 9.0 };
            let radius = (base_radius * scale).clamp(4.0, 28.0);

            // Hit Testing
            if let Some(_cp) = click_pos.filter(|cp| cp.distance(*pos_2d) <= radius * 1.5) {
                clicked_star_id = Some(star.id.clone());
            }

            // Depth Fog Factor (dim stars deep in space)
            let fog_factor = (1.0 - (z_cam - 150.0) / 700.0).clamp(0.25, 1.0);
            let alpha = (255.0 * fog_factor) as u8;

            // Outer Aura Glow
            let pulse =
                ((time_sec * 2.5 + star.activity_level * 5.0).sin() * 0.5 + 0.5) * 0.4 + 0.6;
            painter.circle_filled(
                *pos_2d,
                radius * (1.6 + (1.0 - fog_factor) * 0.4) * pulse,
                Color32::from_rgba_unmultiplied(
                    star.color.r(),
                    star.color.g(),
                    star.color.b(),
                    (45.0 * fog_factor) as u8,
                ),
            );

            // Core Solid Star Node
            let star_color = Color32::from_rgba_unmultiplied(
                star.color.r(),
                star.color.g(),
                star.color.b(),
                alpha,
            );
            painter.circle_filled(*pos_2d, radius, star_color);

            // Selection Highlight Rings & Shockwave Ripple
            if is_selected {
                // Dynamic expanding shockwave ripple
                let ripple_phase = (time_sec * 1.8) % 1.0;
                let ripple_radius = radius + (ripple_phase * 22.0 * scale);
                let ripple_alpha = ((1.0 - ripple_phase) * 160.0) as u8;
                painter.circle_stroke(
                    *pos_2d,
                    ripple_radius,
                    Stroke::new(
                        1.5,
                        Color32::from_rgba_unmultiplied(
                            theme.accent().r(),
                            theme.accent().g(),
                            theme.accent().b(),
                            ripple_alpha,
                        ),
                    ),
                );

                painter.circle_stroke(
                    *pos_2d,
                    radius + 5.0 * scale,
                    Stroke::new(2.0, Color32::from_rgb(255, 255, 255)),
                );
                painter.circle_stroke(
                    *pos_2d,
                    radius + 9.0 * scale,
                    Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(
                            theme.accent().r(),
                            theme.accent().g(),
                            theme.accent().b(),
                            180,
                        ),
                    ),
                );
            }

            // Node Name Label
            let text_color = if is_selected {
                Color32::WHITE
            } else {
                Color32::from_rgba_unmultiplied(220, 230, 245, (200.0 * fog_factor) as u8)
            };
            let font_size = (11.0 * scale).clamp(9.0, 14.0);

            painter.text(
                *pos_2d + Vec2::new(radius + 4.0, -font_size * 0.5),
                egui::Align2::LEFT_CENTER,
                &star.name,
                egui::FontId::proportional(font_size),
                text_color,
            );
        }

        if let Some(id) = clicked_star_id {
            state.selected_galaxy_star_id = Some(id);
        }

        ui.add_space(8.0);

        // 7. Interactive Specialist Detail Inspector Panel
        if let Some(star) = state
            .selected_galaxy_star_id
            .as_ref()
            .and_then(|sel_id| star_map.get(sel_id))
        {
            egui::Frame::group(ui.style())
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0, theme.border_color()))
                .corner_radius(CornerRadius::same(6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (r, g, b) = (star.color.r(), star.color.g(), star.color.b());
                        ui.label(
                            egui::RichText::new("✦")
                                .color(Color32::from_rgb(r, g, b))
                                .size(20.0)
                                .strong(),
                        );
                        ui.heading(
                            egui::RichText::new(&star.name)
                                .color(Color32::from_rgb(r, g, b))
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Mastery: Lv. {}",
                                ((star.domain_opcode % 10) + 1)
                            ))
                            .color(Color32::from_rgb(255, 215, 0))
                            .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!("Role: {}", star.category))
                                .color(theme.accent()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new("✨ Ready")
                                    .color(Color32::from_rgb(100, 220, 140))
                                    .strong(),
                            );
                        });
                    });

                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&star.description).italics());
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Synaptic Links:").strong());
                        for target in &star.connected_to {
                            if let Some(target_star) = star_map.get(target)
                                && ui.button(format!("🔗 {}", target_star.name)).clicked()
                            {
                                state.selected_galaxy_star_id = Some(target.clone());
                            }
                        }
                    });
                });
        }
    }
}
