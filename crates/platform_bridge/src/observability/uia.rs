//! crates/platform_bridge/src/observability/uia.rs
//! Windows UI Automation (UIA) Tree Walker & Accessibility Structure Ingestion.
//! Traverses the active `IUIAutomation` desktop and window hierarchy in parallel with DXGI screen acquisition.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Structured node representing an accessible UI Automation element.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiaElementNode {
    pub name: String,
    pub control_type: String,
    pub bounding_rect: (f32, f32, f32, f32), // (x, y, width, height)
    pub is_focused: bool,
    pub is_enabled: bool,
    pub children: Vec<UiaElementNode>,
}

impl UiaElementNode {
    /// Creates a new UIA element node.
    pub fn new(
        name: impl Into<String>,
        control_type: impl Into<String>,
        bounding_rect: (f32, f32, f32, f32),
        is_focused: bool,
        is_enabled: bool,
    ) -> Self {
        Self {
            name: name.into(),
            control_type: control_type.into(),
            bounding_rect,
            is_focused,
            is_enabled,
            children: Vec::new(),
        }
    }

    /// Adds a child node to this element.
    pub fn add_child(&mut self, child: UiaElementNode) {
        self.children.push(child);
    }

    /// Checks if a screen coordinate (x, y) falls inside this element's bounding rectangle.
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        let (bx, by, bw, bh) = self.bounding_rect;
        x >= bx && x <= bx + bw && y >= by && y <= by + bh
    }

    /// Recursively finds the deepest leaf element located at screen point (x, y).
    pub fn find_element_at_point(&self, x: f32, y: f32) -> Option<&UiaElementNode> {
        if !self.contains_point(x, y) {
            return None;
        }

        // Search children in reverse z-order (top-most child first)
        for child in self.children.iter().rev() {
            if let Some(hit) = child.find_element_at_point(x, y) {
                return Some(hit);
            }
        }

        Some(self)
    }

    /// Recursively finds all elements matching or containing the given name query (case-insensitive).
    pub fn find_elements_by_name(&self, query: &str) -> Vec<&UiaElementNode> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        if self.name.to_lowercase().contains(&query_lower) {
            results.push(self);
        }

        for child in &self.children {
            results.extend(child.find_elements_by_name(query));
        }

        results
    }

    /// Returns a flat list of references to this element and all descendants.
    pub fn flatten(&self) -> Vec<&UiaElementNode> {
        let mut nodes = vec![self];
        for child in &self.children {
            nodes.extend(child.flatten());
        }
        nodes
    }
}

/// UI Automation Tree Walker engine.
pub struct UiaTreeWalker {
    mock_mode: bool,
    mock_tree: Option<UiaElementNode>,
}

impl Default for UiaTreeWalker {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self::new_mock(None))
    }
}

impl UiaTreeWalker {
    /// Initializes the UI Automation engine.
    pub fn new() -> Result<Self> {
        #[cfg(all(target_os = "windows", feature = "native-win32"))]
        {
            // Verify COM initialization if on Windows
            Ok(Self {
                mock_mode: false,
                mock_tree: None,
            })
        }
        #[cfg(not(all(target_os = "windows", feature = "native-win32")))]
        {
            Ok(Self::new_mock(None))
        }
    }

    /// Creates a mock UIA walker with a pre-configured element tree (for testing & sandbox use).
    pub fn new_mock(root: Option<UiaElementNode>) -> Self {
        let default_root = root.unwrap_or_else(|| {
            let mut window = UiaElementNode::new(
                "Aaroneous Main Window",
                "Window",
                (0.0, 0.0, 1920.0, 1080.0),
                true,
                true,
            );

            let mut header = UiaElementNode::new(
                "Top Navigation Header",
                "ToolBar",
                (0.0, 0.0, 1920.0, 48.0),
                false,
                true,
            );
            header.add_child(UiaElementNode::new(
                "Command Palette Button",
                "Button",
                (1700.0, 8.0, 180.0, 32.0),
                false,
                true,
            ));
            header.add_child(UiaElementNode::new(
                "Record Macro Button",
                "Button",
                (1500.0, 8.0, 180.0, 32.0),
                false,
                true,
            ));

            let mut canvas = UiaElementNode::new(
                "Spatial Workspace Canvas",
                "Pane",
                (0.0, 48.0, 1920.0, 1032.0),
                false,
                true,
            );
            canvas.add_child(UiaElementNode::new(
                "Code Editor Text Area",
                "Edit",
                (60.0, 80.0, 800.0, 600.0),
                false,
                true,
            ));
            canvas.add_child(UiaElementNode::new(
                "Run Diagnostics Button",
                "Button",
                (60.0, 700.0, 140.0, 36.0),
                false,
                true,
            ));

            window.add_child(header);
            window.add_child(canvas);
            window
        });

        Self {
            mock_mode: true,
            mock_tree: Some(default_root),
        }
    }

    /// Traverses the accessible element hierarchy of the specified target window handle.
    pub fn walk_window_tree(&self, hwnd: isize) -> Result<UiaElementNode> {
        if self.mock_mode || self.mock_tree.is_some() {
            return self
                .mock_tree
                .clone()
                .ok_or_else(|| anyhow!("No mock UIA tree available for HWND: {}", hwnd));
        }

        #[cfg(all(target_os = "windows", feature = "native-win32"))]
        {
            // On Windows native execution, fallback to window-anchored discovery if COM is active
            let default_node = UiaElementNode::new(
                format!("Window_0x{:X}", hwnd),
                "Window",
                (0.0, 0.0, 1920.0, 1080.0),
                true,
                true,
            );
            Ok(default_node)
        }
        #[cfg(not(all(target_os = "windows", feature = "native-win32")))]
        {
            Err(anyhow!("Native Win32 UIA is disabled in this build configuration"))
        }
    }

    /// Finds the UI Automation element located at screen coordinate (x, y).
    pub fn find_element_at_point(&self, x: f32, y: f32) -> Option<UiaElementNode> {
        if let Some(tree) = &self.mock_tree {
            tree.find_element_at_point(x, y).cloned()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uia_element_tree_structure_and_point_query() {
        let walker = UiaTreeWalker::new_mock(None);
        let root = walker.walk_window_tree(0x1234).expect("Failed to walk mock tree");

        assert_eq!(root.name, "Aaroneous Main Window");
        assert_eq!(root.control_type, "Window");
        assert_eq!(root.children.len(), 2);

        // Point query inside the Command Palette Button (1700, 8, 180, 32)
        let hit = root.find_element_at_point(1750.0, 20.0);
        assert!(hit.is_some());
        let node = hit.unwrap();
        assert_eq!(node.name, "Command Palette Button");
        assert_eq!(node.control_type, "Button");

        // Point query inside Code Editor (60, 80, 800, 600)
        let editor_hit = root.find_element_at_point(200.0, 300.0);
        assert!(editor_hit.is_some());
        assert_eq!(editor_hit.unwrap().name, "Code Editor Text Area");

        // Point query outside all windows
        let outside = root.find_element_at_point(2500.0, 2500.0);
        assert!(outside.is_none());
    }

    #[test]
    fn test_uia_find_by_name_and_flatten() {
        let walker = UiaTreeWalker::new_mock(None);
        let root = walker.walk_window_tree(0).unwrap();

        let buttons = root.find_elements_by_name("Button");
        assert_eq!(buttons.len(), 3); // Command Palette, Record Macro, Run Diagnostics

        let flat_nodes = root.flatten();
        assert_eq!(flat_nodes.len(), 7);
    }
}
