use anyhow::Result;

/// SEMANTIC-03: Edge-Compute Tagger Specialist
/// 
/// High-speed heuristics and zero-shot NLP classification to automatically
/// tag incoming sensory data (window titles, UI automation trees) so the 
/// ProjectionRouter can seamlessly hot-swap .si cartridges without user input.
pub struct EdgeComputeTagger {
    pub confidence_threshold: f32,
}

impl EdgeComputeTagger {
    pub fn new() -> Self {
        Self {
            confidence_threshold: 0.85,
        }
    }

    /// Fast heuristic tagger based on window titles and process names
    pub fn auto_tag_fascia(&self, process_name: &str, window_title: &str) -> Vec<String> {
        let mut tags = Vec::new();
        let p = process_name.to_lowercase();
        let w = window_title.to_lowercase();

        // Developer Heuristics
        if p.contains("code") || p.contains("wezterm") || p.contains("alacritty") || w.contains("visual studio") || w.contains("nvim") {
            tags.push("#dev".to_string());
            tags.push("#rust".to_string());
        }

        // Gaming Heuristics
        if p.contains("steam") || p.contains("obs64") || w.contains("elden ring") || p.contains("cyberpunk") {
            tags.push("#gaming".to_string());
            tags.push("#immersion".to_string());
        }

        // Research Heuristics
        if p.contains("chrome") || p.contains("firefox") || p.contains("edge") {
            tags.push("#research".to_string());
            if w.contains("youtube") {
                tags.push("#media".to_string());
            }
        }

        tags
    }
}