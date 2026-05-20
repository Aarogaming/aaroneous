use egui::{Color32, Painter, Rect, Response, Sense, Ui, Vec2};
use serde::{Serialize, Deserialize};
use crate::{ConstellationNode, NodeType, SpatialCoord};

/// Compute-driven metrics for constellation nodes
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub entropy: f64,          // Shannon entropy of node state
    pub confidence: f64,       // Bayesian confidence score (0.0-1.0)
    pub metabolic_risk: f64,   // Monte Carlo predicted risk (0.0-1.0)
    pub centrality: f64,       // Graph centrality score
    pub mdp_value: f64,        // MDP state value estimate
}

/// Interactive 2D Constellation Renderer for egui
pub struct ConstellationCanvas {
    pub nodes: Vec<ConstellationNode>,
    pub metrics: Vec<NodeMetrics>,
    pub zoom: f32,
    pub pan: Vec2,
    pub selected: Option<usize>,
    pub show_metrics: bool,
}

impl ConstellationCanvas {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            metrics: Vec::new(),
            zoom: 1.0,
            pan: Vec2::ZERO,
            selected: None,
            show_metrics: false,
        }
    }

    /// Update metrics for a specific node from compute engine results
    pub fn update_node_metrics(&mut self, node_index: usize, metrics: NodeMetrics) {
        if node_index < self.nodes.len() {
            while self.metrics.len() <= node_index {
                self.metrics.push(NodeMetrics::default());
            }
            self.metrics[node_index] = metrics;
        }
    }

    /// Compute entropy-based coloring for a node
    pub fn entropy_color(&self, node_index: usize) -> Color32 {
        if node_index < self.metrics.len() {
            let entropy = self.metrics[node_index].entropy;
            let normalized = (entropy / 5.0).clamp(0.0, 1.0); // Max entropy ~5 bits
            let r = (normalized * 255.0) as u8;
            let g = ((1.0 - normalized) * 200.0) as u8;
            Color32::from_rgb(r, g, 50)
        } else {
            Color32::GRAY
        }
    }

    /// Get confidence indicator for a node
    pub fn confidence_indicator(&self, node_index: usize) -> (f64, Color32) {
        if node_index < self.metrics.len() {
            let conf = self.metrics[node_index].confidence;
            let color = if conf > 0.8 {
                Color32::from_rgb(0, 255, 100)
            } else if conf > 0.5 {
                Color32::from_rgb(255, 200, 0)
            } else {
                Color32::from_rgb(255, 80, 80)
            };
            (conf, color)
        } else {
            (0.5, Color32::GRAY)
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size(),
            Sense::click_and_drag(),
        );

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }

        if response.dragged() {
            self.pan += response.drag_delta();
        }

        let painter = Painter::new(ui.ctx().clone(), ui.layer_id(), rect);
        self.render(&painter, rect);

        response
    }

    fn render(&self, painter: &Painter, rect: Rect) {
        // Background grid
        self.draw_grid(painter, rect);

        // Draw edges first (behind nodes)
        for (i, node_a) in self.nodes.iter().enumerate() {
            for (j, node_b) in self.nodes.iter().enumerate() {
                if i >= j { continue; }
                if self.are_connected(node_a, node_b) {
                    let p1 = self.to_screen(&node_a.spatial_coord, rect);
                    let p2 = self.to_screen(&node_b.spatial_coord, rect);
                    
                    // Edge thickness based on relationship strength
                    let strength = if i < self.metrics.len() && j < self.metrics.len() {
                        let conf_a = self.metrics[i].confidence;
                        let conf_b = self.metrics[j].confidence;
                        ((conf_a + conf_b) / 2.0 * 3.0).max(0.5) as f32
                    } else {
                        1.0f32
                    };
                    
                    painter.line_segment([p1, p2], (strength, Color32::DARK_GRAY));
                }
            }
        }

        // Draw nodes
        for (i, node) in self.nodes.iter().enumerate() {
            let pos = self.to_screen(&node.spatial_coord, rect);
            let is_selected = self.selected == Some(i);
            let color = match node.node_type {
                NodeType::Feature => Color32::from_rgb(0, 180, 255),
                NodeType::Bug => Color32::from_rgb(255, 80, 80),
                NodeType::Roadmap => Color32::from_rgb(255, 200, 0),
                NodeType::Decision => Color32::from_rgb(200, 100, 255),
                NodeType::Lore => Color32::from_rgb(100, 255, 100),
                NodeType::Architecture => Color32::from_rgb(0, 255, 200),
                NodeType::Incident => Color32::from_rgb(255, 100, 50),
                NodeType::Reference => Color32::from_rgb(150, 150, 255),
                NodeType::Resource => Color32::from_rgb(255, 150, 200),
                NodeType::TestCase => Color32::from_rgb(200, 255, 150),
            };

            let base_radius = if is_selected { 12.0 } else { 8.0 };
            
            // Adjust radius based on MDP value if available
            let radius = if i < self.metrics.len() {
                let mdp_boost = (self.metrics[i].mdp_value.abs() * 4.0).min(6.0);
                base_radius + mdp_boost as f32
            } else {
                base_radius
            };
            
            painter.circle_filled(pos, radius, color);
            if is_selected {
                painter.circle_stroke(pos, radius + 2.0, (2.0, Color32::WHITE));
            }

            // Show confidence ring around node
            if i < self.metrics.len() {
                let (conf, conf_color) = self.confidence_indicator(i);
                let ring_radius = radius + 4.0;
                painter.circle_stroke(pos, ring_radius, (1.5, conf_color));
            }

            // Label
            painter.text(
                pos + Vec2::new(0.0, radius + 8.0),
                egui::Align2::CENTER_TOP,
                &node.title,
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );

            // Show compute metrics overlay if enabled
            if self.show_metrics && i < self.metrics.len() {
                let m = &self.metrics[i];
                let metric_text = format!(
                    "H: {:.2} | C: {:.2} | R: {:.2}",
                    m.entropy, m.confidence, m.metabolic_risk
                );
                painter.text(
                    pos + Vec2::new(0.0, radius + 22.0),
                    egui::Align2::CENTER_TOP,
                    &metric_text,
                    egui::FontId::monospace(8.0),
                    Color32::LIGHT_GRAY,
                );
            }
        }
    }

    fn draw_grid(&self, painter: &Painter, rect: Rect) {
        let spacing = 40.0 * self.zoom;
        let offset = Vec2::new(self.pan.x % spacing, self.pan.y % spacing);
        let mut x = rect.left() + offset.x;
        while x < rect.right() {
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                (0.5, Color32::from_gray(40)),
            );
            x += spacing;
        }
        let mut y = rect.top() + offset.y;
        while y < rect.bottom() {
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                (0.5, Color32::from_gray(40)),
            );
            y += spacing;
        }
    }

    fn to_screen(&self, coord: &SpatialCoord, rect: Rect) -> egui::Pos2 {
        let center = rect.center();
        egui::pos2(
            center.x + (coord.x as f32 * self.zoom + self.pan.x),
            center.y + (coord.y as f32 * self.zoom + self.pan.y),
        )
    }

    fn are_connected(&self, a: &ConstellationNode, b: &ConstellationNode) -> bool {
        // Simplified: connect if within threshold distance
        let dx = a.spatial_coord.x - b.spatial_coord.x;
        let dy = a.spatial_coord.y - b.spatial_coord.y;
        (dx * dx + dy * dy).sqrt() < 300.0
    }
}
