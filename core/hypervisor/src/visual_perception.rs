/// Visual perception systems for screen immunity, action tokenization,
/// and topological UI graph routing.

// ── Adversarial UI Perturbation Defense ──────────────────────────────
// Noise filter for the SIMD XOR-Delta loop; strips adversarial pixel
// changes that would confuse the mouse engine.

#[derive(Debug, Clone)]
pub struct VisualImmunity {
    pub window_size: usize,
    pub threshold: u8,
}

impl VisualImmunity {
    pub fn new(window: usize, threshold: u8) -> Self {
        VisualImmunity {
            window_size: window,
            threshold,
        }
    }

    /// Filter a delta buffer: zero out isolated pixel changes below
    /// the spatial density threshold within a window.
    /// Returns number of noise pixels removed.
    pub fn filter_delta(&self, delta: &mut [u8], width: u32) -> usize {
        if delta.is_empty() || width == 0 {
            return 0;
        }
        let height = (delta.len() as u32) / width;
        let half = (self.window_size / 2) as i32;
        let mut removed = 0usize;

        // For each pixel, count changed neighbors in window
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                if idx >= delta.len() || delta[idx] == 0 {
                    continue;
                }
                let mut neighbor_count = 0u32;
                let mut total = 0u32;
                for dy in -half..=half {
                    for dx in -half..=half {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                            let nidx = (ny as u32 * width + nx as u32) as usize;
                            total += 1;
                            if nidx < delta.len() && delta[nidx] != 0 {
                                neighbor_count += 1;
                            }
                        }
                    }
                }
                // If density of changes in neighborhood is below threshold, suppress
                let density = if total > 0 {
                    (neighbor_count as f32) / (total as f32)
                } else {
                    0.0
                };
                if density < (self.threshold as f32) / 255.0 {
                    delta[idx] = 0;
                    removed += 1;
                }
            }
        }
        removed
    }
}

// ── Multi-Modal Action-Space Tokenization ────────────────────────────
// Groups raw events into structured multi-hot token patterns.

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActionEvent {
    pub event_type: u8, // 0=mouse_move, 1=mouse_click, 2=key_press, 3=key_release
    pub x: u16,
    pub y: u16,
    pub button: u8,
    pub key_code: u16,
}

#[derive(Debug, Clone)]
pub struct ActionVocab {
    pub tokens: Vec<u64>,
    pub window_size: usize,
}

impl ActionVocab {
    pub fn new(window: usize) -> Self {
        ActionVocab {
            tokens: Vec::new(),
            window_size: window,
        }
    }

    /// Encode an action event into a multi-hot token pattern.
    /// Bits: [0-3] event_type, [4-19] x, [20-35] y, [36-43] button, [44-59] key_code
    pub fn encode(event: &ActionEvent) -> u64 {
        (event.event_type as u64) << 60
            | (event.x as u64) << 44
            | (event.y as u64) << 28
            | (event.button as u64) << 20
            | (event.key_code as u64)
    }

    /// Decode a token back into an event.
    pub fn decode(token: u64) -> ActionEvent {
        ActionEvent {
            event_type: ((token >> 60) & 0xF) as u8,
            x: ((token >> 44) & 0xFFFF) as u16,
            y: ((token >> 28) & 0xFFFF) as u16,
            button: ((token >> 20) & 0xFF) as u8,
            key_code: (token & 0xFFFFF) as u16,
        }
    }

    /// Sliding window tokenization: emit multi-hot patterns.
    pub fn tokenize(&mut self, events: &[ActionEvent]) {
        self.tokens.clear();
        for chunk in events.chunks(self.window_size) {
            let mut pattern = 0u64;
            for event in chunk {
                pattern |= Self::encode(event);
            }
            self.tokens.push(pattern);
        }
    }

    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }
}

// ── Topological UI Graph Routing ─────────────────────────────────────
// App as a mesh of active geometric coordinate buttons; navigate via
// shortest-path graph calculations instead of image templates.

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UIElement {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub action: u64,
}

#[derive(Debug, Clone)]
pub struct UISurfaceMap {
    pub elements: Vec<UIElement>,
    pub edges: Vec<(u32, u32, f32)>, // (from_id, to_id, cost)
}

impl Default for UISurfaceMap {
    fn default() -> Self {
        Self::new()
    }
}

impl UISurfaceMap {
    pub fn new() -> Self {
        UISurfaceMap {
            elements: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_element(&mut self, id: u32, x: f32, y: f32, w: f32, h: f32, action: u64) {
        self.elements.push(UIElement {
            id,
            x,
            y,
            w,
            h,
            action,
        });
    }

    /// Build edges between elements based on spatial proximity.
    /// Cost = Euclidean distance between centers.
    pub fn build_proximity_edges(&mut self) {
        self.edges.clear();
        for i in 0..self.elements.len() {
            for j in (i + 1)..self.elements.len() {
                let a = &self.elements[i];
                let b = &self.elements[j];
                let cx1 = a.x + a.w / 2.0;
                let cy1 = a.y + a.h / 2.0;
                let cx2 = b.x + b.w / 2.0;
                let cy2 = b.y + b.h / 2.0;
                let cost = ((cx1 - cx2).powi(2) + (cy1 - cy2).powi(2)).sqrt();
                self.edges.push((a.id, b.id, cost));
                self.edges.push((b.id, a.id, cost));
            }
        }
    }

    /// Greedy shortest path from start_id to target_id.
    /// Returns list of element IDs in order.
    pub fn navigate(&self, start_id: u32, target_id: u32) -> Vec<u32> {
        if start_id == target_id {
            return vec![start_id];
        }
        // Dijkstra
        let n = self.elements.len();
        let mut dist = vec![f32::MAX; n];
        let mut prev = vec![None; n];
        let mut visited = vec![false; n];
        let idx_of = |id: u32| -> Option<usize> { self.elements.iter().position(|e| e.id == id) };

        let start_idx = match idx_of(start_id) {
            Some(i) => i,
            None => return vec![],
        };
        let target_idx = match idx_of(target_id) {
            Some(i) => i,
            None => return vec![],
        };

        dist[start_idx] = 0.0;

        for _ in 0..n {
            let u = (0..n)
                .filter(|i| !visited[*i])
                .min_by(|&a, &b| dist[a].partial_cmp(&dist[b]).unwrap())
                .unwrap_or(0);
            if visited[u] {
                break;
            }
            visited[u] = true;
            if u == target_idx {
                break;
            }

            let u_id = self.elements[u].id;
            for &(from, to, cost) in &self.edges {
                if from == u_id
                    && let Some(v) = idx_of(to)
                {
                    let new = dist[u] + cost;
                    if new < dist[v] {
                        dist[v] = new;
                        prev[v] = Some(u);
                    }
                }
            }
        }

        // Reconstruct path
        let mut path = Vec::new();
        let mut cur = target_idx;
        while cur != start_idx {
            path.push(self.elements[cur].id);
            match prev[cur] {
                Some(p) => cur = p,
                None => return vec![],
            }
        }
        path.push(self.elements[start_idx].id);
        path.reverse();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_immunity_all_same() {
        let immunity = VisualImmunity::new(3, 200);
        let mut delta = vec![0u8; 64];
        delta[0] = 255; // isolated pixel
        let removed = immunity.filter_delta(&mut delta, 8);
        assert!(removed > 0, "isolated pixel should be filtered");
        assert_eq!(delta[0], 0);
    }

    #[test]
    fn test_visual_immunity_dense_region() {
        let immunity = VisualImmunity::new(3, 10);
        let mut delta = vec![0u8; 64];
        // Fill a 3x3 block
        for y in 0..3 {
            for x in 0..3 {
                delta[(y * 8 + x) as usize] = 255;
            }
        }
        let removed = immunity.filter_delta(&mut delta, 8);
        // Dense region should be preserved
        assert!(
            delta[1 * 8 + 1] != 0,
            "center of dense region should survive"
        );
    }

    #[test]
    fn test_action_vocab_encode_decode() {
        let event = ActionEvent {
            event_type: 1,
            x: 100,
            y: 200,
            button: 0,
            key_code: 0,
        };
        let token = ActionVocab::encode(&event);
        let decoded = ActionVocab::decode(token);
        assert_eq!(decoded.event_type, 1);
        assert_eq!(decoded.x, 100);
        assert_eq!(decoded.y, 200);
    }

    #[test]
    fn test_action_vocab_tokenize() {
        let mut vocab = ActionVocab::new(3);
        let events = vec![
            ActionEvent {
                event_type: 0,
                x: 10,
                y: 20,
                button: 0,
                key_code: 0,
            },
            ActionEvent {
                event_type: 1,
                x: 30,
                y: 40,
                button: 1,
                key_code: 0,
            },
        ];
        vocab.tokenize(&events);
        assert_eq!(vocab.token_count(), 1);
    }

    #[test]
    fn test_ui_surface_map_navigate() {
        let mut map = UISurfaceMap::new();
        map.add_element(1, 0.0, 0.0, 10.0, 10.0, 0xAA);
        map.add_element(2, 100.0, 0.0, 10.0, 10.0, 0xBB);
        map.add_element(3, 200.0, 0.0, 10.0, 10.0, 0xCC);
        map.build_proximity_edges();
        let path = map.navigate(1, 3);
        assert!(path.len() >= 2, "path too short: {:?}", path);
        assert_eq!(path[0], 1, "should start at element 1");
        assert_eq!(path[path.len() - 1], 3, "should end at element 3");
    }

    #[test]
    fn test_ui_surface_map_same_element() {
        let mut map = UISurfaceMap::new();
        map.add_element(1, 0.0, 0.0, 10.0, 10.0, 0xAA);
        let path = map.navigate(1, 1);
        assert_eq!(path, vec![1]);
    }

    #[test]
    fn test_ui_surface_map_no_path() {
        let map = UISurfaceMap::new();
        let path = map.navigate(1, 2);
        assert!(path.is_empty());
    }
}
