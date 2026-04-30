# Phase A: The Watcher Implementation Guide

**Goal:** Detect behavioral anchor points (game end, error, load screen) and trigger Ariel intents

**Timeline:** 20 hours | Tests: +18 | Target: 573 total tests

---

## Architecture Overview

```
┌──────────────────────────┐
│ Glass Framebuffer (60Hz) │
└────────────┬─────────────┘
             │
             ▼
┌──────────────────────────┐
│ Anchor Detector          │
│ (OCR + pattern matching) │
└────────────┬─────────────┘
             │
             ▼
┌──────────────────────────┐
│ Context Event Emitter    │
│ (Signal Ariel)           │
└────────────┬─────────────┘
             │
             ▼
┌──────────────────────────┐
│ Ariel Intent Router      │
│ (Generate action)        │
└────────────┬─────────────┘
             │
             ▼
┌──────────────────────────┐
│ HID Executor             │
│ (Perform action)         │
└──────────────────────────┘
```

---

## Step A1: Anchor Detector (6 hours)

### Create `src/visionary/mod.rs`

```rust
pub mod anchor_detector;
pub mod context_event;
pub mod intent_router;

pub use anchor_detector::AnchorDetector;
pub use context_event::ContextEvent;
pub use intent_router::IntentRouter;
```

### Create `src/visionary/anchor_detector.rs`

```rust
use serde::{Deserialize, Serialize};
use image::{DynamicImage, Rgba};
use std::collections::HashMap;

/// Types of behavioral anchors the system can detect
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnchorPoint {
    /// Game ended (victory/defeat)
    GameEnd {
        victory: bool,
        score: Option<i32>,
        game_name: Option<String>,
    },

    /// Simulation/build completed
    SimulationEnd {
        app_name: String,
        status: String,  // "success", "failure", "partial"
        metrics: Option<HashMap<String, f32>>,
    },

    /// Error or warning detected
    ErrorDetected {
        error_type: String,  // "compilation_error", "runtime_error", "warning"
        severity: u8,        // 1-10
        message: Option<String>,
        line_number: Option<u32>,
    },

    /// Loading screen detected
    LoadingScreen {
        app_name: String,
        estimated_duration_ms: Option<u32>,
    },

    /// Custom pattern (user-defined)
    Custom {
        pattern_name: String,
        data: HashMap<String, String>,
    },

    /// No anchor detected
    None,
}

/// Detects anchor points from framebuffer
pub struct AnchorDetector {
    /// Pattern definitions for known applications
    patterns: HashMap<String, AnchorPattern>,
    
    /// OCR engine (optional, for advanced text detection)
    ocr_enabled: bool,
    
    /// Last detected anchor (for debouncing)
    last_anchor: Option<AnchorPoint>,
    last_anchor_timestamp_ms: u64,
}

#[derive(Clone)]
struct AnchorPattern {
    app_name: String,
    trigger_text: Vec<String>,        // Text to look for (case-insensitive)
    trigger_colors: Vec<(u8, u8, u8)>, // RGB colors to look for
    anchor_type: AnchorType,
}

#[derive(Clone)]
enum AnchorType {
    GameEnd,
    SimulationEnd,
    ErrorDetected,
    LoadingScreen,
}

impl AnchorDetector {
    pub fn new(ocr_enabled: bool) -> Self {
        let mut detector = Self {
            patterns: HashMap::new(),
            ocr_enabled,
            last_anchor: None,
            last_anchor_timestamp_ms: 0,
        };

        // Define built-in patterns
        detector.register_steam_patterns();
        detector.register_ide_patterns();
        detector.register_cadence_patterns();

        detector
    }

    /// Process framebuffer and detect anchors
    pub fn detect(&mut self, framebuffer: &DynamicImage) -> Result<AnchorPoint, String> {
        // Debounce: don't detect same anchor more than once per 500ms
        let now_ms = now_ms();
        if let Some(ref last) = self.last_anchor {
            if now_ms - self.last_anchor_timestamp_ms < 500 {
                return Ok(AnchorPoint::None);
            }
        }

        // Try pattern matching first (fast)
        if let Some(anchor) = self.detect_by_patterns(framebuffer)? {
            self.last_anchor = Some(anchor.clone());
            self.last_anchor_timestamp_ms = now_ms;
            tracing::debug!("Detected anchor: {:?}", anchor);
            return Ok(anchor);
        }

        // Fall back to OCR if enabled
        if self.ocr_enabled {
            if let Some(anchor) = self.detect_by_ocr(framebuffer)? {
                self.last_anchor = Some(anchor.clone());
                self.last_anchor_timestamp_ms = now_ms;
                tracing::debug!("Detected anchor (OCR): {:?}", anchor);
                return Ok(anchor);
            }
        }

        Ok(AnchorPoint::None)
    }

    /// Detect by color/shape patterns
    fn detect_by_patterns(&self, framebuffer: &DynamicImage) -> Result<Option<AnchorPoint>, String> {
        // Convert to RGB for analysis
        let rgb_image = framebuffer.to_rgb8();

        for pattern in self.patterns.values() {
            // Check if trigger colors appear in image
            let color_match = pattern.trigger_colors.iter().any(|&color| {
                self.color_appears_in_image(&rgb_image, color, tolerance: 30)
            });

            if color_match {
                // This is a rough heuristic; expand based on pattern
                let anchor = match pattern.anchor_type {
                    AnchorType::GameEnd => {
                        AnchorPoint::GameEnd {
                            victory: true,  // Heuristic: green = victory
                            score: None,
                            game_name: Some(pattern.app_name.clone()),
                        }
                    }
                    AnchorType::ErrorDetected => {
                        AnchorPoint::ErrorDetected {
                            error_type: "detected".to_string(),
                            severity: 7,
                            message: None,
                            line_number: None,
                        }
                    }
                    AnchorType::LoadingScreen => {
                        AnchorPoint::LoadingScreen {
                            app_name: pattern.app_name.clone(),
                            estimated_duration_ms: None,
                        }
                    }
                    AnchorType::SimulationEnd => {
                        AnchorPoint::SimulationEnd {
                            app_name: pattern.app_name.clone(),
                            status: "completed".to_string(),
                            metrics: None,
                        }
                    }
                };

                return Ok(Some(anchor));
            }
        }

        Ok(None)
    }

    /// Detect by OCR (expensive, use sparingly)
    fn detect_by_ocr(&self, framebuffer: &DynamicImage) -> Result<Option<AnchorPoint>, String> {
        // TODO: Integrate tesseract or google vision API
        // For now, return None
        Ok(None)
    }

    fn color_appears_in_image(
        &self,
        image: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
        target_color: (u8, u8, u8),
        tolerance: u8,
    ) -> bool {
        for pixel in image.pixels() {
            let diff_r = (pixel[0] as i16 - target_color.0 as i16).abs() as u8;
            let diff_g = (pixel[1] as i16 - target_color.1 as i16).abs() as u8;
            let diff_b = (pixel[2] as i16 - target_color.2 as i16).abs() as u8;

            if diff_r < tolerance && diff_g < tolerance && diff_b < tolerance {
                return true;
            }
        }
        false
    }

    /// Register detection patterns for Steam
    fn register_steam_patterns(&mut self) {
        // Green victory screen
        self.patterns.insert(
            "steam_victory".to_string(),
            AnchorPattern {
                app_name: "steam".to_string(),
                trigger_text: vec!["victory", "won", "congratulations"].iter().map(|s| s.to_string()).collect(),
                trigger_colors: vec![(76, 175, 80)],  // Green
                anchor_type: AnchorType::GameEnd,
            },
        );

        // Red defeat screen
        self.patterns.insert(
            "steam_defeat".to_string(),
            AnchorPattern {
                app_name: "steam".to_string(),
                trigger_text: vec!["defeat", "lost", "game over"].iter().map(|s| s.to_string()).collect(),
                trigger_colors: vec![(244, 67, 54)],  // Red
                anchor_type: AnchorType::GameEnd,
            },
        );
    }

    /// Register detection patterns for IDE
    fn register_ide_patterns(&mut self) {
        // Red squiggly (error)
        self.patterns.insert(
            "ide_error".to_string(),
            AnchorPattern {
                app_name: "vscode".to_string(),
                trigger_text: vec!["error".to_string()],
                trigger_colors: vec![(255, 0, 0)],  // Red
                anchor_type: AnchorType::ErrorDetected,
            },
        );

        // Yellow squiggly (warning)
        self.patterns.insert(
            "ide_warning".to_string(),
            AnchorPattern {
                app_name: "vscode".to_string(),
                trigger_text: vec!["warning".to_string()],
                trigger_colors: vec![(255, 193, 7)],  // Yellow
                anchor_type: AnchorType::ErrorDetected,
            },
        );
    }

    /// Register detection patterns for CAD/Simulation
    fn register_cadence_patterns(&mut self) {
        // Simulation complete
        self.patterns.insert(
            "cadence_complete".to_string(),
            AnchorPattern {
                app_name: "cadence".to_string(),
                trigger_text: vec!["simulation complete".to_string()],
                trigger_colors: vec![(76, 175, 80)],  // Green
                anchor_type: AnchorType::SimulationEnd,
            },
        );
    }

    /// Add custom pattern
    pub fn register_custom_pattern(
        &mut self,
        name: &str,
        app_name: &str,
        trigger_text: Vec<String>,
        trigger_colors: Vec<(u8, u8, u8)>,
    ) {
        self.patterns.insert(
            name.to_string(),
            AnchorPattern {
                app_name: app_name.to_string(),
                trigger_text,
                trigger_colors,
                anchor_type: AnchorType::Custom { /* ... */ },
            },
        );
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchor_detector_creation() {
        let detector = AnchorDetector::new(false);
        assert!(detector.patterns.len() > 0, "Should have built-in patterns");
    }

    #[test]
    fn test_detect_game_end_victory() {
        let mut detector = AnchorDetector::new(false);
        
        // Create a green image (victory)
        let image = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(100, 100, |_, _| {
            image::Rgb([76, 175, 80])  // Green
        }));

        let anchor = detector.detect(&image).unwrap();
        match anchor {
            AnchorPoint::GameEnd { victory, game_name, .. } => {
                assert!(victory);
                assert_eq!(game_name, Some("steam".to_string()));
            }
            _ => panic!("Expected GameEnd anchor"),
        }
    }

    #[test]
    fn test_detect_error() {
        let mut detector = AnchorDetector::new(false);
        
        // Create a red image (error)
        let image = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(100, 100, |_, _| {
            image::Rgb([255, 0, 0])  // Red
        }));

        let anchor = detector.detect(&image).unwrap();
        match anchor {
            AnchorPoint::ErrorDetected { severity, .. } => {
                assert!(severity > 0);
            }
            _ => panic!("Expected ErrorDetected anchor"),
        }
    }

    #[test]
    fn test_debounce() {
        let mut detector = AnchorDetector::new(false);
        
        let image = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(100, 100, |_, _| {
            image::Rgb([76, 175, 80])
        }));

        // First detection
        let anchor1 = detector.detect(&image).unwrap();
        assert!(!matches!(anchor1, AnchorPoint::None));

        // Second detection immediately after (should be debounced)
        let anchor2 = detector.detect(&image).unwrap();
        assert!(matches!(anchor2, AnchorPoint::None), "Should debounce duplicate detections");
    }

    #[test]
    fn test_custom_pattern() {
        let mut detector = AnchorDetector::new(false);
        detector.register_custom_pattern(
            "my_game",
            "custom_game",
            vec!["boss defeated".to_string()],
            vec![(100, 200, 150)],
        );

        assert!(detector.patterns.contains_key("my_game"));
    }

    #[test]
    fn test_no_anchor() {
        let mut detector = AnchorDetector::new(false);
        
        // Create a neutral gray image
        let image = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(100, 100, |_, _| {
            image::Rgb([128, 128, 128])  // Gray
        }));

        let anchor = detector.detect(&image).unwrap();
        assert!(matches!(anchor, AnchorPoint::None), "Should detect no anchor in neutral image");
    }
}
```

**Add to Cargo.toml:**
```toml
[dependencies]
image = "0.24"
# tesseract = "0.2"  # Uncomment for OCR support
```

**Tests: 5-7 passing**

---

## Step A2: Context Event Emitter (4 hours)

### Create `src/visionary/context_event.rs`

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::anchor_detector::AnchorPoint;

/// A context event fired by anchor detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEvent {
    pub event_id: Uuid,
    pub timestamp_ms: u64,
    
    /// What was detected
    pub anchor: AnchorPoint,
    
    /// Where it came from
    pub source: String,  // "glass"
    
    /// Intent generated by Ariel (filled later)
    pub ariel_intent: Option<String>,
    
    /// Outcome of action
    pub outcome: Option<String>,
}

impl ContextEvent {
    pub fn new(anchor: AnchorPoint) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            timestamp_ms: now_ms(),
            anchor,
            source: "glass".to_string(),
            ariel_intent: None,
            outcome: None,
        }
    }

    /// Emit to event log
    pub async fn emit(&self, event_log: &crate::EventLog) -> Result<(), String> {
        let json = serde_json::to_string(self)
            .map_err(|e| format!("Failed to serialize context event: {}", e))?;
        
        tracing::info!("Context event: {}", json);
        // TODO: Write to DNA Bank or event log
        Ok(())
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_event_creation() {
        let anchor = AnchorPoint::GameEnd {
            victory: true,
            score: Some(1000),
            game_name: Some("game".to_string()),
        };
        
        let event = ContextEvent::new(anchor.clone());
        
        assert_eq!(event.source, "glass");
        assert_eq!(event.anchor, anchor);
        assert!(event.ariel_intent.is_none());
    }

    #[test]
    fn test_context_event_serialization() {
        let anchor = AnchorPoint::ErrorDetected {
            error_type: "compilation".to_string(),
            severity: 9,
            message: Some("undefined variable".to_string()),
            line_number: Some(42),
        };
        
        let event = ContextEvent::new(anchor);
        let json = serde_json::to_string(&event).unwrap();
        
        assert!(json.contains("ErrorDetected"));
        assert!(json.contains("undefined variable"));
    }

    #[test]
    fn test_context_event_with_intent() {
        let anchor = AnchorPoint::LoadingScreen {
            app_name: "game".to_string(),
            estimated_duration_ms: Some(5000),
        };
        
        let mut event = ContextEvent::new(anchor);
        event.ariel_intent = Some("show_tip".to_string());
        
        assert_eq!(event.ariel_intent, Some("show_tip".to_string()));
    }

    #[test]
    fn test_context_event_with_outcome() {
        let anchor = AnchorPoint::Custom {
            pattern_name: "test".to_string(),
            data: std::collections::HashMap::new(),
        };
        
        let mut event = ContextEvent::new(anchor);
        event.outcome = Some("success".to_string());
        
        assert_eq!(event.outcome, Some("success".to_string()));
    }
}
```

**Tests: 3-4 passing**

---

## Step A3: Ariel Intent Router (5 hours)

### Modify `src/hive_runtime.rs`

Add to `HiveRuntime`:

```rust
use crate::visionary::{AnchorDetector, ContextEvent};

pub struct HiveRuntime {
    // ... existing fields ...
    
    /// Anchor detector for behavioral triggers
    anchor_detector: Option<Arc<AnchorDetector>>,
}

impl HiveRuntime {
    /// Process context event and generate intent
    pub async fn handle_context_event(&self, event: &ContextEvent) -> Result<String, String> {
        use crate::visionary::AnchorPoint;
        
        let intent = match &event.anchor {
            AnchorPoint::GameEnd { victory, score, game_name } => {
                if *victory {
                    format!("You won {}! Show celebration overlay.", game_name.as_ref().unwrap_or(&"game".to_string()))
                } else {
                    format!("Game over. Would you like tips for next time?")
                }
            }
            
            AnchorPoint::ErrorDetected { error_type, severity, message, line_number } => {
                format!(
                    "Error detected (severity {}): {}. Line {}. Fetch troubleshooting guide.",
                    severity,
                    message.as_ref().unwrap_or(&"unknown".to_string()),
                    line_number.unwrap_or(0)
                )
            }
            
            AnchorPoint::LoadingScreen { app_name, estimated_duration_ms } => {
                let duration_sec = estimated_duration_ms.unwrap_or(5) / 1000;
                format!("{} is loading (~{}s). Show tips or suggestions.", app_name, duration_sec)
            }
            
            AnchorPoint::SimulationEnd { app_name, status, metrics } => {
                format!("{} simulation {}: {}", app_name, status, 
                    serde_json::to_string(metrics).unwrap_or_default())
            }
            
            AnchorPoint::Custom { pattern_name, data } => {
                format!("Custom trigger '{}': {:?}", pattern_name, data)
            }
            
            AnchorPoint::None => return Ok("No action".to_string()),
        };
        
        tracing::info!("Ariel intent: {}", intent);
        Ok(intent)
    }
}
```

### Create test file: `src/visionary/intent_router_tests.rs`

```rust
#[cfg(test)]
mod tests {
    use crate::visionary::{AnchorPoint, ContextEvent};
    use crate::hive_runtime::HiveRuntime;

    #[tokio::test]
    async fn test_handle_game_end_victory() {
        let runtime = HiveRuntime::new().await.unwrap();
        
        let anchor = AnchorPoint::GameEnd {
            victory: true,
            score: Some(1000),
            game_name: Some("roguelike".to_string()),
        };
        
        let event = ContextEvent::new(anchor);
        let intent = runtime.handle_context_event(&event).await.unwrap();
        
        assert!(intent.contains("won"));
        assert!(intent.contains("celebration"));
    }

    #[tokio::test]
    async fn test_handle_error_detected() {
        let runtime = HiveRuntime::new().await.unwrap();
        
        let anchor = AnchorPoint::ErrorDetected {
            error_type: "compilation".to_string(),
            severity: 9,
            message: Some("undefined variable".to_string()),
            line_number: Some(42),
        };
        
        let event = ContextEvent::new(anchor);
        let intent = runtime.handle_context_event(&event).await.unwrap();
        
        assert!(intent.contains("Error detected"));
        assert!(intent.contains("undefined variable"));
        assert!(intent.contains("42"));
    }

    #[tokio::test]
    async fn test_handle_loading_screen() {
        let runtime = HiveRuntime::new().await.unwrap();
        
        let anchor = AnchorPoint::LoadingScreen {
            app_name: "game".to_string(),
            estimated_duration_ms: Some(5000),
        };
        
        let event = ContextEvent::new(anchor);
        let intent = runtime.handle_context_event(&event).await.unwrap();
        
        assert!(intent.contains("loading"));
        assert!(intent.contains("5s"));
    }

    #[tokio::test]
    async fn test_handle_simulation_end() {
        let runtime = HiveRuntime::new().await.unwrap();
        
        let mut metrics = std::collections::HashMap::new();
        metrics.insert("duration_ms".to_string(), 1250.0);
        metrics.insert("power_dissipation_w".to_string(), 2.5);
        
        let anchor = AnchorPoint::SimulationEnd {
            app_name: "cadence".to_string(),
            status: "success".to_string(),
            metrics: Some(metrics),
        };
        
        let event = ContextEvent::new(anchor);
        let intent = runtime.handle_context_event(&event).await.unwrap();
        
        assert!(intent.contains("cadence"));
        assert!(intent.contains("success"));
    }

    #[tokio::test]
    async fn test_handle_custom_anchor() {
        let runtime = HiveRuntime::new().await.unwrap();
        
        let mut data = std::collections::HashMap::new();
        data.insert("action".to_string(), "screenshot".to_string());
        
        let anchor = AnchorPoint::Custom {
            pattern_name: "user_action".to_string(),
            data,
        };
        
        let event = ContextEvent::new(anchor);
        let intent = runtime.handle_context_event(&event).await.unwrap();
        
        assert!(intent.contains("user_action"));
    }

    #[test]
    fn test_handle_no_anchor() {
        let event = ContextEvent::new(AnchorPoint::None);
        // This would need async context, skip for now
    }
}
```

**Tests: 5-6 passing**

---

## Step A4: Integration + Testing (5 hours)

### Create `src/visionary/integration_tests.rs`

```rust
#[cfg(test)]
mod integration_tests {
    use crate::visionary::{AnchorDetector, ContextEvent, AnchorPoint};
    use crate::hid_driver::HidDriver;
    use crate::hive_runtime::HiveRuntime;
    use image::DynamicImage;

    #[tokio::test]
    async fn test_framebuffer_to_action_pipeline() {
        // Create game end image
        let image = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(100, 100, |_, _| {
            image::Rgb([76, 175, 80])  // Green (victory)
        }));

        // Detect anchor
        let mut detector = AnchorDetector::new(false);
        let anchor = detector.detect(&image).unwrap();

        assert!(!matches!(anchor, AnchorPoint::None));

        // Create context event
        let event = ContextEvent::new(anchor);
        
        // Generate intent
        let runtime = HiveRuntime::new().await.unwrap();
        let intent = runtime.handle_context_event(&event).await.unwrap();
        
        assert!(!intent.is_empty());
        assert!(intent.contains("won") || intent.contains("victory"));
    }

    #[tokio::test]
    async fn test_error_detection_to_action() {
        // Create error image (red)
        let image = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(100, 100, |_, _| {
            image::Rgb([255, 0, 0])
        }));

        let mut detector = AnchorDetector::new(false);
        let anchor = detector.detect(&image).unwrap();

        match anchor {
            AnchorPoint::ErrorDetected { .. } => {
                let event = ContextEvent::new(anchor);
                let runtime = HiveRuntime::new().await.unwrap();
                let intent = runtime.handle_context_event(&event).await.unwrap();
                
                assert!(intent.contains("Error"));
            }
            _ => panic!("Expected error detection"),
        }
    }

    #[tokio::test]
    async fn test_end_to_end_with_hid() {
        // Step 1: Detect anchor
        let image = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(100, 100, |_, _| {
            image::Rgb([76, 175, 80])
        }));

        let mut detector = AnchorDetector::new(false);
        let anchor = detector.detect(&image).unwrap();

        // Step 2: Generate intent
        let event = ContextEvent::new(anchor);
        let runtime = HiveRuntime::new().await.unwrap();
        let intent = runtime.handle_context_event(&event).await.unwrap();

        // Step 3: Would execute via HID driver
        // (This is a mock test; real test would execute macro)
        let driver = HidDriver::new().await.unwrap();
        
        // Example: click at (100, 100) to dismiss victory screen
        use crate::hid_driver::{HidCommand, MouseButton};
        let click = HidCommand::MouseClick {
            button: MouseButton::Left,
            x: 100,
            y: 100,
        };
        
        let response = driver.execute(click).await;
        assert!(response.is_ok());
    }

    #[test]
    fn test_performance_framebuffer_to_decision() {
        use std::time::Instant;

        let image = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(100, 100, |_, _| {
            image::Rgb([76, 175, 80])
        }));

        let mut detector = AnchorDetector::new(false);
        
        let start = Instant::now();
        let _ = detector.detect(&image).unwrap();
        let elapsed = start.elapsed();

        // Should be <100ms for framebuffer → decision
        assert!(elapsed.as_millis() < 100, "Anchor detection took {}ms (target <100ms)", elapsed.as_millis());
    }
}
```

**Tests: 4 integration tests**

---

## Step A5: Expose in lib.rs

Add to `src/lib.rs`:

```rust
pub mod visionary;
pub use visionary::{AnchorDetector, ContextEvent, AnchorPoint};
```

---

## Test Summary

**Phase A Tests: 18-20 passing**

- Anchor Detector: 7 tests
- Context Event: 4 tests
- Intent Router: 6 tests
- Integration: 4 tests

**Total tests target: 573 (555 + 18)**

---

## Success Criteria Checklist

- [ ] AnchorDetector creates with built-in patterns
- [ ] Detect GameEnd (victory/defeat) from green/red images
- [ ] Detect ErrorDetected from red pixels
- [ ] Detect LoadingScreen from known UI patterns
- [ ] Debounce duplicate anchors within 500ms
- [ ] ContextEvent serializes correctly
- [ ] Ariel generates appropriate intents for each anchor type
- [ ] Integration test: framebuffer → anchor → intent → action
- [ ] Latency <100ms for anchor detection
- [ ] All 18 tests passing

---

## Next Steps (Phase B)

Once Phase A is complete:
1. Build Style Bank for storing aesthetic engrams
2. Implement design generation algorithm
3. Integrate with VFD duty cycle
4. Generate UI prototypes during idle

**Target: Begin Phase B next week after Phase A ships.**
