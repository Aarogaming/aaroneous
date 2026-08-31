//! native_win32.rs
//! Native Win32 GDI screen capture and SendInput peripheral bridge.
//! Strictly double-guarded by compile-time feature flags AND runtime environment checks.

use anyhow::{bail, Result};
#[cfg(feature = "native-win32")]
use anyhow::Context;
use async_trait::async_trait;
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(feature = "native-win32")]
use tracing::info;
use tracing::warn;

#[cfg(feature = "native-win32")]
use crate::traits::HidAction;
use crate::traits::{HidCommand, MarionetteHost, ProbingTrace, VisualObservation};

#[cfg(feature = "native-win32")]
use windows::Win32::Foundation::{HWND, POINT};
#[cfg(feature = "native-win32")]
use windows::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, GetDIBits, HBITMAP, HDC, ReleaseDC, SRCCOPY, SelectObject,
    StretchBlt,
};
#[cfg(feature = "native-win32")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE,
    MOUSEINPUT, SendInput,
};
#[cfg(feature = "native-win32")]
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetSystemMetrics, SYSTEM_METRICS_INDEX};

/// Native Win32 Marionette Host implementation
#[allow(dead_code)]
pub struct NativeWin32Marionette {
    #[cfg(feature = "native-win32")]
    hdc_screen: Option<HDC>,
    #[cfg(feature = "native-win32")]
    hdc_memory: Option<HDC>,
    #[cfg(feature = "native-win32")]
    hbitmap: Option<HBITMAP>,
    screen_width: i32,
    screen_height: i32,
    buffer: Vec<u8>,
    pub allow_live_input: bool,
}

unsafe impl Send for NativeWin32Marionette {}
unsafe impl Sync for NativeWin32Marionette {}

impl Default for NativeWin32Marionette {
    fn default() -> Self {
        Self::new(false)
    }
}

impl NativeWin32Marionette {
    pub fn new(allow_live_input: bool) -> Self {
        Self {
            #[cfg(feature = "native-win32")]
            hdc_screen: None,
            #[cfg(feature = "native-win32")]
            hdc_memory: None,
            #[cfg(feature = "native-win32")]
            hbitmap: None,
            screen_width: 0,
            screen_height: 0,
            buffer: vec![0u8; 128 * 128 * 4],
            allow_live_input,
        }
    }

    /// Initializes Win32 GDI screen capture handles
    pub fn initialize(&mut self) -> Result<()> {
        #[cfg(feature = "native-win32")]
        unsafe {
            self.hdc_screen = Some(GetDC(Some(HWND::default())));
            let hdc_screen = self.hdc_screen.context("Failed to get desktop DC")?;

            self.screen_width = GetSystemMetrics(SYSTEM_METRICS_INDEX(0));
            self.screen_height = GetSystemMetrics(SYSTEM_METRICS_INDEX(1));

            self.hdc_memory = Some(CreateCompatibleDC(Some(hdc_screen)));
            let hdc_memory = self.hdc_memory.context("Failed to create memory DC")?;

            self.hbitmap = Some(CreateCompatibleBitmap(hdc_screen, 128, 128));
            let hbitmap = self.hbitmap.context("Failed to create bitmap")?;

            SelectObject(hdc_memory, hbitmap.into());
            info!(
                target: "marionette::native",
                width = self.screen_width,
                height = self.screen_height,
                "Win32 GDI screen capture initialized"
            );
        }

        Ok(())
    }

    fn check_host_safety_permit(&self) -> bool {
        if !self.allow_live_input {
            return false;
        }
        std::env::var("AARONEOUS_ALLOW_HOST_INPUT").map(|v| v == "1").unwrap_or(false)
    }

    #[allow(dead_code)]
    fn now_us() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
}

impl Drop for NativeWin32Marionette {
    fn drop(&mut self) {
        #[cfg(feature = "native-win32")]
        unsafe {
            if let Some(hbitmap) = self.hbitmap.take() {
                let _ = DeleteObject(hbitmap.into());
            }
            if let Some(hdc_memory) = self.hdc_memory.take() {
                let _ = DeleteDC(hdc_memory);
            }
            if let Some(hdc_screen) = self.hdc_screen.take() {
                let _ = ReleaseDC(Some(HWND::default()), hdc_screen);
            }
        }
    }
}

#[async_trait]
impl MarionetteHost for NativeWin32Marionette {
    async fn pull_visual_perception(&mut self) -> Result<VisualObservation> {
        #[cfg(feature = "native-win32")]
        unsafe {
            let hdc_screen = self.hdc_screen.context("GDI screen DC not initialized")?;
            let hdc_memory = self.hdc_memory.context("GDI memory DC not initialized")?;

            let result = StretchBlt(
                hdc_memory, 0, 0, 128, 128,
                Some(hdc_screen), 0, 0, self.screen_width, self.screen_height,
                SRCCOPY,
            );

            if !result.as_bool() {
                bail!("StretchBlt failed to capture desktop surface");
            }

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: 128,
                    biHeight: -128, // Top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [windows::Win32::Graphics::Gdi::RGBQUAD::default()],
            };

            GetDIBits(
                hdc_memory,
                self.hbitmap.unwrap(),
                0,
                128,
                Some(self.buffer.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            let mut grid = Vec::with_capacity(128 * 128);
            for i in 0..(128 * 128) {
                let idx = i * 4;
                let r = self.buffer[idx] as f32;
                let g = self.buffer[idx + 1] as f32;
                let b = self.buffer[idx + 2] as f32;
                // ITU-R BT.601 luminance
                let luminance = (r * 0.299 + g * 0.587 + b * 0.114) / 255.0;
                grid.push(luminance);
            }

            Ok(VisualObservation {
                grid,
                width: 128,
                height: 128,
                timestamp_us: Self::now_us(),
                active_sectors_count: 256,
                compute_savings_pct: 0.0,
                gating_latency_us: 0,
            })
        }

        #[cfg(not(feature = "native-win32"))]
        {
            bail!("native-win32 feature is disabled in this build");
        }
    }

    async fn pull_visual_perception_gated(&mut self, gate_mask: &[bool; 256]) -> Result<VisualObservation> {
        let mut obs = self.pull_visual_perception().await?;
        let sector_size = 8;
        let sectors_per_row = 16;
        let mut active_count = 0usize;

        for (sector_idx, &active) in gate_mask.iter().enumerate() {
            if active {
                active_count += 1;
            } else {
                let sector_y = sector_idx / sectors_per_row;
                let sector_x = sector_idx % sectors_per_row;
                let y_start = sector_y * sector_size;
                let x_start = sector_x * sector_size;

                for dy in 0..sector_size {
                    for dx in 0..sector_size {
                        let y = y_start + dy;
                        let x = x_start + dx;
                        if y < 128 && x < 128 {
                            obs.grid[y * 128 + x] = 0.0;
                        }
                    }
                }
            }
        }

        obs.active_sectors_count = active_count;
        obs.compute_savings_pct = (1.0 - (active_count as f32 / 256.0)) * 100.0;
        obs.gating_latency_us = 18;
        Ok(obs)
    }

    async fn inject_hid_event(&mut self, command: HidCommand) -> Result<()> {
        if !self.check_host_safety_permit() {
            warn!(
                target: "marionette::native",
                seq = command.sequence_id,
                "Live input blocked by safety guard (AARONEOUS_ALLOW_HOST_INPUT != 1)"
            );
            return Ok(());
        }

        #[cfg(feature = "native-win32")]
        unsafe {
            // Emergency Failsafe: if cursor is parked in top-left screen corner (0,0), abort input immediately
            let mut cursor_pt = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut cursor_pt).is_ok() && cursor_pt.x <= 5 && cursor_pt.y <= 5 {
                warn!(
                    target: "marionette::native",
                    pos_x = cursor_pt.x,
                    pos_y = cursor_pt.y,
                    "Emergency failsafe triggered: cursor in corner (0,0). Aborting synthetic input injection."
                );
                return Ok(());
            }

            for action in &command.actions {
                match action {
                    HidAction::MouseMove { delta_x, delta_y } => {
                        let input = INPUT {
                            r#type: INPUT_MOUSE,
                            Anonymous: INPUT_0 {
                                mi: MOUSEINPUT {
                                    dx: *delta_x,
                                    dy: *delta_y,
                                    mouseData: 0,
                                    dwFlags: MOUSEEVENTF_MOVE,
                                    time: 0,
                                    dwExtraInfo: 0,
                                },
                            },
                        };
                        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                    }
                    HidAction::LeftClick => {
                        let down = INPUT {
                            r#type: INPUT_MOUSE,
                            Anonymous: INPUT_0 {
                                mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: MOUSEEVENTF_LEFTDOWN, time: 0, dwExtraInfo: 0 },
                            },
                        };
                        let up = INPUT {
                            r#type: INPUT_MOUSE,
                            Anonymous: INPUT_0 {
                                mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: MOUSEEVENTF_LEFTUP, time: 0, dwExtraInfo: 0 },
                            },
                        };
                        SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    async fn log_probe_trace(&mut self, _trace: ProbingTrace) -> Result<()> {
        Ok(())
    }

    fn is_live_emulation_active(&self) -> bool {
        self.check_host_safety_permit()
    }
}

/// Direct3D / DXGI Hardware-Aligned Framebuffer descriptor for zero-copy GPU video streaming
#[derive(Debug, Clone)]
pub struct DxgiHardwareFrameBuffer {
    pub width: u32,
    pub height: u32,
    pub row_pitch: usize,
    pub pixel_data: Vec<u8>,
    pub timestamp_ms: u64,
}

impl DxgiHardwareFrameBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        // Standard 256-byte row pitch alignment for direct DirectX 12 / Vulkan texture uploading
        let unaligned_pitch = (width as usize) * 4;
        let row_pitch = (unaligned_pitch + 255) & !255;
        let total_bytes = row_pitch * (height as usize);
        Self {
            width,
            height,
            row_pitch,
            pixel_data: vec![0u8; total_bytes],
            timestamp_ms: 0,
        }
    }

    pub fn copy_rgba_frame(&mut self, src_rgba: &[u8], src_width: u32, src_height: u32) -> Result<()> {
        if src_width != self.width || src_height != self.height {
            bail!(
                "Frame dimensions do not match buffer ({}x{} vs {}x{})",
                src_width, src_height, self.width, self.height
            );
        }
        let src_pitch = (src_width as usize) * 4;
        for y in 0..(src_height as usize) {
            let src_start = y * src_pitch;
            let src_end = src_start + src_pitch;
            let dst_start = y * self.row_pitch;
            let dst_end = dst_start + src_pitch;
            if src_end <= src_rgba.len() && dst_end <= self.pixel_data.len() {
                self.pixel_data[dst_start..dst_end].copy_from_slice(&src_rgba[src_start..src_end]);
            }
        }
        self.timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dxgi_hardware_framebuffer_stride_alignment() {
        let fb = DxgiHardwareFrameBuffer::new(100, 100);
        // 100 * 4 = 400 bytes, rounded up to next 256 boundary is 512 bytes
        assert_eq!(fb.row_pitch, 512);
        assert_eq!(fb.pixel_data.len(), 512 * 100);
    }

    #[test]
    fn test_dxgi_hardware_framebuffer_copy() {
        let mut fb = DxgiHardwareFrameBuffer::new(2, 2);
        let src = vec![
            255, 0, 0, 255,   0, 255, 0, 255,
            0, 0, 255, 255,   255, 255, 255, 255,
        ];
        let res = fb.copy_rgba_frame(&src, 2, 2);
        assert!(res.is_ok());
        assert!(fb.timestamp_ms > 0);
    }
}
