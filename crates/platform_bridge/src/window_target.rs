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

/// Native Win32 Window Styles & Manipulation Pipeline for Detached Transparent Overlay HUDs (SHELL-03)
pub struct TransparentWindowPipeline;

impl TransparentWindowPipeline {
    /// Pure bitwise helper to compute updated extended style bits with WS_EX_TRANSPARENT and WS_EX_LAYERED.
    #[inline]
    pub fn calculate_overlay_style(current_style: isize, click_through: bool) -> isize {
        const WS_EX_TRANSPARENT_BIT: isize = 0x0000_0020;
        const WS_EX_LAYERED_BIT: isize = 0x0008_0000;

        if click_through {
            current_style | WS_EX_TRANSPARENT_BIT | WS_EX_LAYERED_BIT
        } else {
            // Remove transparent bit while keeping layered bit for translucent rendering
            (current_style & !WS_EX_TRANSPARENT_BIT) | WS_EX_LAYERED_BIT
        }
    }

    /// Locate top-level window belonging to the current process.
    pub fn find_own_window() -> Result<Option<isize>> {
        #[cfg(all(target_os = "windows", feature = "native-win32"))]
        {
            Self::find_own_window_native()
        }
        #[cfg(not(all(target_os = "windows", feature = "native-win32")))]
        {
            Ok(Some(0x1337))
        }
    }

    #[cfg(all(target_os = "windows", feature = "native-win32"))]
    fn find_own_window_native() -> Result<Option<isize>> {
        use windows::core::BOOL;
        use windows::Win32::Foundation::{HWND, LPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
        };

        struct FindContext {
            target_pid: u32,
            found_hwnd: Option<HWND>,
        }

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam.0 as *mut FindContext);
            let mut proc_id = 0u32;
            let _ = GetWindowThreadProcessId(hwnd, Some(&mut proc_id));

            if proc_id == ctx.target_pid && IsWindowVisible(hwnd).as_bool() {
                ctx.found_hwnd = Some(hwnd);
                return BOOL(0); // Stop enumeration, window found
            }
            BOOL(1) // Continue
        }

        let mut ctx = FindContext {
            target_pid: std::process::id(),
            found_hwnd: None,
        };

        unsafe {
            let _ = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut _ as isize));
        }

        Ok(ctx.found_hwnd.map(|h| h.0 as isize))
    }

    /// Sets or unsets Win32 WS_EX_TRANSPARENT and WS_EX_LAYERED extended styles on the target window handle.
    pub fn apply_click_through(hwnd: isize, click_through: bool) -> Result<()> {
        if hwnd == 0 {
            anyhow::bail!("Invalid null HWND passed to apply_click_through");
        }

        #[cfg(all(target_os = "windows", feature = "native-win32"))]
        {
            Self::apply_click_through_native(hwnd, click_through)
        }
        #[cfg(not(all(target_os = "windows", feature = "native-win32")))]
        {
            let _ = (hwnd, click_through);
            Ok(())
        }
    }

    #[cfg(all(target_os = "windows", feature = "native-win32"))]
    fn apply_click_through_native(hwnd: isize, click_through: bool) -> Result<()> {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE,
        };

        let handle = HWND(hwnd as *mut core::ffi::c_void);
        unsafe {
            let current = GetWindowLongPtrW(handle, GWL_EXSTYLE);
            let updated = Self::calculate_overlay_style(current, click_through);
            let ret = SetWindowLongPtrW(handle, GWL_EXSTYLE, updated);
            if ret == 0 && current != updated {
                let err = std::io::Error::last_os_error();
                tracing::warn!(target: "hud::win32", error = %err, "SetWindowLongPtrW notice");
            }
        }
        Ok(())
    }

    /// Query whether a given window currently has WS_EX_TRANSPARENT style bit enabled.
    pub fn is_click_through(hwnd: isize) -> Result<bool> {
        if hwnd == 0 {
            anyhow::bail!("Invalid null HWND passed to is_click_through");
        }

        #[cfg(all(target_os = "windows", feature = "native-win32"))]
        {
            Self::is_click_through_native(hwnd)
        }
        #[cfg(not(all(target_os = "windows", feature = "native-win32")))]
        {
            let _ = hwnd;
            Ok(false)
        }
    }

    #[cfg(all(target_os = "windows", feature = "native-win32"))]
    fn is_click_through_native(hwnd: isize) -> Result<bool> {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWL_EXSTYLE};

        let handle = HWND(hwnd as *mut core::ffi::c_void);
        let current = unsafe { GetWindowLongPtrW(handle, GWL_EXSTYLE) };
        const WS_EX_TRANSPARENT_BIT: isize = 0x0000_0020;
        Ok((current & WS_EX_TRANSPARENT_BIT) != 0)
    }

    /// Sets or clears topmost z-order flag (HWND_TOPMOST / HWND_NOTOPMOST).
    pub fn set_always_on_top(hwnd: isize, on_top: bool) -> Result<()> {
        if hwnd == 0 {
            anyhow::bail!("Invalid null HWND passed to set_always_on_top");
        }

        #[cfg(all(target_os = "windows", feature = "native-win32"))]
        {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            };

            let handle = HWND(hwnd as *mut core::ffi::c_void);
            let z_order = if on_top { HWND_TOPMOST } else { HWND_NOTOPMOST };
            unsafe {
                let _ = SetWindowPos(
                    handle,
                    Some(z_order),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            Ok(())
        }
        #[cfg(not(all(target_os = "windows", feature = "native-win32")))]
        {
            let _ = (hwnd, on_top);
            Ok(())
        }
    }

    /// Non-blocking check for global F12 toggle hotkey using Win32 GetAsyncKeyState.
    /// Edge-triggered: returns true only on transition from released to pressed.
    pub fn check_global_toggle_hotkey() -> bool {
        #[cfg(all(target_os = "windows", feature = "native-win32"))]
        {
            use std::sync::atomic::{AtomicBool, Ordering};
            use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F12};

            static WAS_DOWN: AtomicBool = AtomicBool::new(false);

            let state = unsafe { GetAsyncKeyState(VK_F12.0 as i32) };
            let is_down = (state as u16 & 0x8000) != 0;
            let was_down = WAS_DOWN.swap(is_down, Ordering::SeqCst);

            is_down && !was_down
        }
        #[cfg(not(all(target_os = "windows", feature = "native-win32")))]
        {
            false
        }
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

    #[test]
    fn test_calculate_overlay_style() {
        // Initial normal window style (no transparent, no layered)
        let initial_style = 0x0000_0000isize;

        // Apply click-through: should set both WS_EX_TRANSPARENT (0x20) and WS_EX_LAYERED (0x80000)
        let click_through_style = TransparentWindowPipeline::calculate_overlay_style(initial_style, true);
        assert_eq!(click_through_style & 0x0000_0020, 0x0000_0020);
        assert_eq!(click_through_style & 0x0008_0000, 0x0008_0000);

        // Revert click-through: should clear WS_EX_TRANSPARENT while retaining WS_EX_LAYERED
        let interactive_style = TransparentWindowPipeline::calculate_overlay_style(click_through_style, false);
        assert_eq!(interactive_style & 0x0000_0020, 0);
        assert_eq!(interactive_style & 0x0008_0000, 0x0008_0000);
    }

    #[test]
    fn test_apply_click_through_null_handle() {
        let res = TransparentWindowPipeline::apply_click_through(0, true);
        assert!(res.is_err());

        let res = TransparentWindowPipeline::is_click_through(0);
        assert!(res.is_err());

        let res = TransparentWindowPipeline::set_always_on_top(0, true);
        assert!(res.is_err());
    }

    #[test]
    fn test_check_global_toggle_hotkey_safety() {
        // Must execute cleanly without crashing or panicking
        let _ = TransparentWindowPipeline::check_global_toggle_hotkey();
    }
}
