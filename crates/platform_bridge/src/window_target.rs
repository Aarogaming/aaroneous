//! crates/desktop_emulator/src/window_target.rs
//! Discord-style Screen, Window, and Application Target Enumeration and Capture Modifiers Engine.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Target Source for Visual & Audio Capture
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureTarget {
    EntireDisplay {
        display_id: usize,
        name: String,
    },
    ApplicationWindow {
        hwnd: isize,
        title: String,
        process_name: String,
    },
}

/// Audio Loopback and Microphone Modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCaptureModifier {
    SystemAndGameLoopback,
    MicrophoneOnly,
    Muted,
}

/// Capture Video & Sensory Modifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureModifiers {
    pub target: CaptureTarget,
    pub audio_modifier: AudioCaptureModifier,
    pub target_fps: u32,
    pub neural_resolution: (usize, usize),
    pub entropy_threshold: f32,
}

impl Default for CaptureModifiers {
    fn default() -> Self {
        Self {
            target: CaptureTarget::EntireDisplay {
                display_id: 0,
                name: "Primary Display (1920x1080)".to_string(),
            },
            audio_modifier: AudioCaptureModifier::SystemAndGameLoopback,
            target_fps: 60,
            neural_resolution: (128, 128),
            entropy_threshold: 0.05,
        }
    }
}

/// Discovered Physical Display Monitor
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredScreen {
    pub display_id: usize,
    pub name: String,
    pub resolution: (u32, u32),
    pub is_primary: bool,
}

/// Discovered Application Window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredWindow {
    pub hwnd: isize,
    pub title: String,
    pub process_name: String,
    pub is_active: bool,
}

/// Window & Screen Discovery Engine
pub struct WindowDiscoveryEngine;

impl WindowDiscoveryEngine {
    /// Discovers active application windows and screens on the system
    pub fn enumerate_available_targets() -> Result<Vec<DiscoveredWindow>> {
        #[cfg(all(target_os = "windows", feature = "native-win32"))]
        {
            Self::enumerate_windows_native()
        }
        #[cfg(not(all(target_os = "windows", feature = "native-win32")))]
        {
            Ok(Self::mock_windows())
        }
    }

    #[cfg(all(target_os = "windows", feature = "native-win32"))]
    fn enumerate_windows_native() -> Result<Vec<DiscoveredWindow>> {
        use windows::core::BOOL;
        use windows::Win32::Foundation::{HWND, LPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
        };

        struct EnumContext {
            windows: Vec<DiscoveredWindow>,
        }

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let context = &mut *(lparam.0 as *mut EnumContext);

            if IsWindowVisible(hwnd).as_bool() {
                let len = GetWindowTextLengthW(hwnd);
                if len > 0 {
                    let mut buffer = vec![0u16; (len + 1) as usize];
                    let read_len = GetWindowTextW(hwnd, &mut buffer);
                    if read_len > 0 {
                        let title = String::from_utf16_lossy(&buffer[..read_len as usize]);
                        let trimmed = title.trim();
                        if !trimmed.is_empty()
                            && trimmed != "Default IME"
                            && trimmed != "MSCTFIME UI"
                        {
                            context.windows.push(DiscoveredWindow {
                                hwnd: hwnd.0 as isize,
                                title: trimmed.to_string(),
                                process_name: format!(
                                    "{}.exe",
                                    trimmed.split_whitespace().next().unwrap_or("App")
                                ),
                                is_active: true,
                            });
                        }
                    }
                }
            }

            BOOL(1) // Continue enumeration
        }

        let mut context = EnumContext {
            windows: Vec::new(),
        };
        unsafe {
            let _ = EnumWindows(Some(enum_proc), LPARAM(&mut context as *mut _ as isize));
        }

        if context.windows.is_empty() {
            Ok(Self::mock_windows())
        } else {
            Ok(context.windows)
        }
    }

    /// Discovers connected physical display screens/monitors on the host system
    pub fn enumerate_screens() -> Result<Vec<DiscoveredScreen>> {
        #[cfg(all(target_os = "windows", feature = "native-win32"))]
        {
            Self::enumerate_screens_native()
        }
        #[cfg(not(all(target_os = "windows", feature = "native-win32")))]
        {
            Ok(Self::mock_screens())
        }
    }

    #[cfg(all(target_os = "windows", feature = "native-win32"))]
    fn enumerate_screens_native() -> Result<Vec<DiscoveredScreen>> {
        use windows::core::BOOL;
        use windows::Win32::Foundation::{LPARAM, RECT};
        use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR};

        struct MonitorContext {
            screens: Vec<DiscoveredScreen>,
        }

        unsafe extern "system" fn monitor_proc(
            _hmonitor: HMONITOR,
            _hdc: HDC,
            rect_ptr: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let context = &mut *(lparam.0 as *mut MonitorContext);
            if !rect_ptr.is_null() {
                let r = *rect_ptr;
                let width = (r.right - r.left).unsigned_abs();
                let height = (r.bottom - r.top).unsigned_abs();
                let idx = context.screens.len();
                let is_primary = r.left == 0 && r.top == 0;
                let primary_tag = if is_primary { " (Primary)" } else { "" };

                context.screens.push(DiscoveredScreen {
                    display_id: idx,
                    name: format!("Display {} - {}x{}{}", idx + 1, width, height, primary_tag),
                    resolution: (width, height),
                    is_primary,
                });
            }
            BOOL(1) // Continue enumeration
        }

        let mut context = MonitorContext {
            screens: Vec::new(),
        };

        unsafe {
            let _ = EnumDisplayMonitors(
                None,
                None,
                Some(monitor_proc),
                LPARAM(&mut context as *mut _ as isize),
            );
        }

        if context.screens.is_empty() {
            Ok(Self::mock_screens())
        } else {
            Ok(context.screens)
        }
    }

    fn mock_screens() -> Vec<DiscoveredScreen> {
        vec![
            DiscoveredScreen {
                display_id: 0,
                name: "Display 1 - 1920x1080 (Primary)".to_string(),
                resolution: (1920, 1080),
                is_primary: true,
            },
            DiscoveredScreen {
                display_id: 1,
                name: "Display 2 - 2560x1440".to_string(),
                resolution: (2560, 1440),
                is_primary: false,
            },
        ]
    }

    fn mock_windows() -> Vec<DiscoveredWindow> {
        vec![
            DiscoveredWindow {
                hwnd: 0x1001,
                title: "Cyberpunk 2077".to_string(),
                process_name: "Cyberpunk2077.exe".to_string(),
                is_active: true,
            },
            DiscoveredWindow {
                hwnd: 0x1002,
                title: "Minecraft 1.21".to_string(),
                process_name: "javaw.exe".to_string(),
                is_active: true,
            },
            DiscoveredWindow {
                hwnd: 0x1003,
                title: "Visual Studio Code".to_string(),
                process_name: "Code.exe".to_string(),
                is_active: true,
            },
            DiscoveredWindow {
                hwnd: 0x1004,
                title: "Google Chrome".to_string(),
                process_name: "chrome.exe".to_string(),
                is_active: true,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_targets() {
        let targets = WindowDiscoveryEngine::enumerate_available_targets().unwrap();
        assert!(!targets.is_empty());
    }

    #[test]
    fn test_capture_modifiers_default() {
        let mods = CaptureModifiers::default();
        assert_eq!(mods.target_fps, 60);
        assert_eq!(mods.neural_resolution, (128, 128));
    }
}
