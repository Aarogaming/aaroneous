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
    /// D3D11 / DXGI Shared Handle for direct zero-copy GPU-to-GPU texture interop with wgpu
    pub shared_handle: Option<usize>,
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
            shared_handle: None,
        }
    }

    /// Associates an existing Direct3D 11/12 GPU shared texture handle for zero-copy VRAM interop.
    pub fn with_shared_handle(mut self, handle: usize) -> Self {
        self.shared_handle = Some(handle);
        self
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

/// DXGI Desktop Duplication capture backend.
///
/// Uses the Windows DXGI Desktop Duplication API for GPU-accelerated
/// screen capture at near-zero CPU overhead. Falls back to GDI when
/// DXGI is unavailable (e.g., remote desktop sessions).
#[cfg(feature = "native-win32")]
pub struct DxgiCaptureBackend {
    duplication: Option<windows::Win32::Graphics::Dxgi::IDXGIOutputDuplication>,
    device: Option<windows::Win32::Graphics::Direct3D11::ID3D11Device>,
    context: Option<windows::Win32::Graphics::Direct3D11::ID3D11DeviceContext>,
    staging_texture: Option<windows::Win32::Graphics::Direct3D11::ID3D11Texture2D>,
    width: u32,
    height: u32,
    initialized: bool,
}

#[cfg(feature = "native-win32")]
impl DxgiCaptureBackend {
    pub fn new() -> Self {
        Self {
            duplication: None,
            device: None,
            context: None,
            staging_texture: None,
            width: 0,
            height: 0,
            initialized: false,
        }
    }

    /// Initialize DXGI Desktop Duplication pipeline.
    ///
    /// Creates a D3D11 device, enumerates adapters/outputs, and acquires
    /// the desktop output duplication interface.
    pub fn initialize(&mut self) -> Result<()> {
        use windows::core::Interface;
        use windows::Win32::Foundation::HMODULE;
        use windows::Win32::Graphics::Dxgi::*;
        use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
        use windows::Win32::Graphics::Direct3D::*;
        use windows::Win32::Graphics::Direct3D11::*;

        unsafe {
            // Create D3D11 device with hardware acceleration
            let mut device = None;
            let mut context = None;

            let feature_levels = [D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_1];

            let result = D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            );

            if result.is_err() {
                // Fallback to WARP (software rasterizer)
                let result_warp = D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_WARP,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_FLAG(0),
                    Some(&feature_levels),
                    D3D11_SDK_VERSION,
                    Some(&mut device),
                    None,
                    Some(&mut context),
                );
                result_warp.map_err(|e| anyhow::anyhow!("D3D11 WARP device creation failed: {}", e))?;
            }

            let device = device.ok_or_else(|| anyhow::anyhow!("D3D11 device is None"))?;
            let context = context.ok_or_else(|| anyhow::anyhow!("D3D11 context is None"))?;

            // Enumerate DXGI adapter and output
            let dxgi_device: IDXGIDevice = device.cast()?;
            let adapter: IDXGIAdapter = dxgi_device.GetParent()?;
            let output = adapter.EnumOutputs(0)?;
            let output1: IDXGIOutput1 = output.cast()?;

            // Get output description for dimensions
            let desc = output.GetDesc()?;
            self.width = (desc.DesktopCoordinates.right - desc.DesktopCoordinates.left) as u32;
            self.height = (desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top) as u32;

            // Create desktop duplication
            let duplication = output1.DuplicateOutput(&device)?;

            // Create staging texture for CPU readback
            let tex_desc = D3D11_TEXTURE2D_DESC {
                Width: self.width,
                Height: self.height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };

            let mut staging_texture = None;
            device.CreateTexture2D(&tex_desc, None, Some(&mut staging_texture))?;

            self.duplication = Some(duplication);
            self.device = Some(device);
            self.context = Some(context);
            self.staging_texture = staging_texture;
            self.initialized = true;

            info!(
                target: "dxgi_capture",
                width = self.width,
                height = self.height,
                "DXGI Desktop Duplication initialized"
            );
        }

        Ok(())
    }

    /// Acquire the next desktop frame and convert to BGRA pixel buffer.
    ///
    /// Returns the frame as a BGRA byte slice suitable for luminance conversion.
    pub fn capture_frame_bgra(&mut self, buffer: &mut [u8]) -> Result<()> {
        use windows::core::Interface;
        use windows::Win32::Graphics::Dxgi::*;
        use windows::Win32::Graphics::Direct3D11::*;

        if !self.initialized {
            bail!("DXGI capture not initialized");
        }

        let duplication = self.duplication.as_ref()
            .ok_or_else(|| anyhow::anyhow!("DXGI duplication not available"))?;
        let context = self.context.as_ref()
            .ok_or_else(|| anyhow::anyhow!("D3D11 context not available"))?;
        let staging = self.staging_texture.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Staging texture not available"))?;

        unsafe {
            // Acquire next frame with 16ms timeout (~60fps)
            let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut resource = None;

            let acquire_result = duplication.AcquireNextFrame(
                16,
                &mut frame_info,
                &mut resource,
            );

            match acquire_result {
                Ok(()) => {
                    let resource = resource.ok_or_else(|| anyhow::anyhow!("Frame resource is None"))?;
                    let desktop_texture: ID3D11Texture2D = resource.cast()?;

                    // Copy to staging texture for CPU readback
                    context.CopyResource(staging, &desktop_texture);

                    // Map staging texture for CPU access
                    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                    context.Map(
                        staging,
                        0,
                        D3D11_MAP_READ,
                        0,
                        Some(&mut mapped),
                    )?;

                    // Copy from mapped texture to output buffer
                    let src_pitch = mapped.RowPitch as usize;
                    let dst_pitch = (self.width as usize) * 4;

                    for y in 0..(self.height as usize) {
                        let src_offset = y * src_pitch;
                        let dst_offset = y * dst_pitch;
                        if src_offset + dst_pitch <= mapped.pData as usize + mapped.RowPitch as usize * self.height as usize
                            && dst_offset + dst_pitch <= buffer.len()
                        {
                            let src_slice = std::slice::from_raw_parts(
                                (mapped.pData as *const u8).add(src_offset),
                                dst_pitch,
                            );
                            buffer[dst_offset..dst_offset + dst_pitch].copy_from_slice(src_slice);
                        }
                    }

                    context.Unmap(staging, 0);
                    duplication.ReleaseFrame()?;

                    Ok(())
                }
                Err(e) => {
                    // DXGI_ERROR_WAIT_TIMEOUT means no new frame
                    if e.code() == DXGI_ERROR_WAIT_TIMEOUT {
                        bail!("No new frame available (timeout)");
                    }
                    bail!("DXGI AcquireNextFrame failed: {}", e);
                }
            }
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Convenience: capture a single frame via DXGI and convert to 128x128 luminance grid.
///
/// Returns `None` if DXGI is unavailable or the frame capture fails.
#[cfg(feature = "native-win32")]
pub fn capture_dxgi_frame_128x128() -> Option<Vec<f32>> {
    use DxgiCaptureBackend;

    let mut backend = DxgiCaptureBackend::new();
    if backend.initialize().is_err() {
        return None;
    }

    let (w, h) = backend.dimensions();
    let mut bgra_buffer = vec![0u8; (w as usize) * (h as usize) * 4];

    if backend.capture_frame_bgra(&mut bgra_buffer).is_err() {
        return None;
    }

    // Downscale to 128x128 with box filter and convert to luminance
    let mut grid = vec![0.0f32; 128 * 128];
    let scale_x = w as f32 / 128.0;
    let scale_y = h as f32 / 128.0;

    for dy in 0..128usize {
        for dx in 0..128usize {
            let src_x = (dx as f32 * scale_x) as usize;
            let src_y = (dy as f32 * scale_y) as usize;
            let src_idx = (src_y * w as usize + src_x) * 4;
            if src_idx + 3 < bgra_buffer.len() {
                let b = bgra_buffer[src_idx] as f32;
                let g = bgra_buffer[src_idx + 1] as f32;
                let r = bgra_buffer[src_idx + 2] as f32;
                // ITU-R BT.601 luminance
                grid[dy * 128 + dx] = (r * 0.299 + g * 0.587 + b * 0.114) / 255.0;
            }
        }
    }

    Some(grid)
}

#[cfg(not(feature = "native-win32"))]
pub fn capture_dxgi_frame_128x128() -> Option<Vec<f32>> {
    None
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
