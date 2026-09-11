// core/hypervisor/src/hud/dashboard.rs
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct DashboardManager {
    pub active_dashboard: String,
    pub dashboards_dir: PathBuf,
    pub visible_dashboards: HashMap<String, bool>,
}

impl DashboardManager {
    pub fn new() -> Self {
        let mut dashboards_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        dashboards_dir.push("aaroneous");
        dashboards_dir.push("dashboards");

        if !dashboards_dir.exists() {
            let _ = fs::create_dir_all(&dashboards_dir);
        }
        
        let mut manager = Self {
            active_dashboard: "Default".to_string(),
            dashboards_dir,
            visible_dashboards: HashMap::new(),
        };

        manager.generate_presets_if_missing();
        manager.sync_visibility_state();

        manager
    }

    fn generate_presets_if_missing(&self) {
        let presets = vec![
            "Default",
            "Programming",
            "Streaming",
            "Gaming",
            "AI_Models",
            "Dev_Tools"
        ];
        
        for preset in presets {
            let mut file_path = self.dashboards_dir.clone();
            file_path.push(format!("{}.ron", preset));
            if !file_path.exists() {
                // Initialize default .ron layouts with placeholder structure
                let layout_str = format!("// Auto-generated preset layout for {}\n", preset);
                let _ = fs::write(file_path, layout_str);
            }
        }
    }

    fn sync_visibility_state(&mut self) {
        // Load all available presets and default them to visible
        let available = self.list_all_dashboards_on_disk();
        for dash in available {
            self.visible_dashboards.entry(dash).or_insert(true);
        }
    }

    pub fn list_all_dashboards_on_disk(&self) -> Vec<String> {
        let mut list = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.dashboards_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".ron") {
                        list.push(name.trim_end_matches(".ron").to_string());
                    }
                }
            }
        }
        list.sort();
        list.dedup();
        list
    }

    /// Lists only the dashboards the user has enabled in settings
    pub fn list_visible_dashboards(&self) -> Vec<String> {
        let mut list: Vec<String> = self.visible_dashboards.iter()
            .filter(|(_, is_visible)| **is_visible)
            .map(|(k, _)| k.clone())
            .collect();
        list.sort();
        
        // Safety Fallback: 0 dashboards = no UI
        if list.is_empty() {
            list.push("Default".to_string());
        }
        
        list
    }

    pub fn toggle_dashboard_visibility(&mut self, name: &str, visible: bool) {
        let current_visible_count = self.visible_dashboards.values().filter(|&&v| v).count();
        
        // Anti-Void Safety Lock: Prevent disabling the very last active dashboard
        if !visible && current_visible_count <= 1 {
            return;
        }
        
        self.visible_dashboards.insert(name.to_string(), visible);
    }

    pub fn save_dashboard(&self, name: &str) {
        let mut file_path = self.dashboards_dir.clone();
        file_path.push(format!("{}.ron", name));
        let layout_str = format!("// Saved layout for {}\n", name);
        let _ = fs::write(file_path, layout_str);
    }

    pub fn load_dashboard(&mut self, name: &str) -> bool {
        let mut file_path = self.dashboards_dir.clone();
        file_path.push(format!("{}.ron", name));
        if file_path.exists() {
            self.active_dashboard = name.to_string();
            return true;
        }
        false
    }
}
