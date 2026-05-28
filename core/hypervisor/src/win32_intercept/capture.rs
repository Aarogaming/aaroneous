// Win32 GDI Screen Capture
// Captures desktop as normalized 128x128 float grid using StretchBlt

use windows::Win32::Foundation::{BOOL, HWND};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
    GetDIBits, GetDC, ReleaseDC, SelectObject, StretchBlt,
    BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HDC, HBITMAP, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SYSTEM_METRICS_INDEX};

pub const GRID_WIDTH: usize = 128;
pub const GRID_HEIGHT: usize = 128;
pub const GRID_SIZE: usize = GRID_WIDTH * GRID_HEIGHT;

/// Captures the Windows desktop as a 128x128 normalized float grid
pub struct Win32ScreenCapture {
    hdc_screen: Option<HDC>,
    hdc_memory: Option<HDC>,
    hbitmap: Option<HBITMAP>,
    screen_width: i32,
    screen_height: i32,
    buffer: Vec<u8>,
}

// Safety: HDC and HBITMAP are thread-safe handles on Windows
unsafe impl Send for Win32ScreenCapture {}
unsafe impl Sync for Win32ScreenCapture {}

impl Win32ScreenCapture {
    pub fn new() -> Self {
        Self {
            hdc_screen: None,
            hdc_memory: None,
            hbitmap: None,
            screen_width: 0,
            screen_height: 0,
            buffer: vec![0u8; GRID_WIDTH * GRID_HEIGHT * 4],
        }
    }

    pub fn initialize(&mut self) -> Result<(), String> {
        unsafe {
            // Get screen DC
            self.hdc_screen = Some(GetDC(HWND::default()));
            let hdc_screen = self.hdc_screen.ok_or("Failed to get screen DC")?;

            // Get screen dimensions
            self.screen_width = GetSystemMetrics(SYSTEM_METRICS_INDEX(0));
            self.screen_height = GetSystemMetrics(SYSTEM_METRICS_INDEX(1));

            // Create compatible DC
            self.hdc_memory = Some(CreateCompatibleDC(hdc_screen));
            let hdc_memory = self.hdc_memory.ok_or("Failed to create memory DC")?;

            // Create compatible bitmap
            self.hbitmap = Some(CreateCompatibleBitmap(hdc_screen, GRID_WIDTH as i32, GRID_HEIGHT as i32));
            let hbitmap = self.hbitmap.ok_or("Failed to create bitmap")?;

            // Select bitmap into memory DC
            SelectObject(hdc_memory, hbitmap);
        }

        Ok(())
    }

    pub fn capture_frame(&mut self) -> Result<Vec<f32>, String> {
        unsafe {
            let hdc_screen = self.hdc_screen.ok_or("Not initialized")?;
            let hdc_memory = self.hdc_memory.ok_or("Not initialized")?;

            // Blit screen to memory DC (stretched to 128x128)
            let result = StretchBlt(
                hdc_memory,
                0, 0,
                GRID_WIDTH as i32, GRID_HEIGHT as i32,
                hdc_screen,
                0, 0,
                self.screen_width, self.screen_height,
                SRCCOPY,
            );

            if result == BOOL(0) {
                return Err("StretchBlt failed".to_string());
            }

            // Get bitmap bits
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: GRID_WIDTH as i32,
                    biHeight: -(GRID_HEIGHT as i32), // Top-down
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
                GRID_HEIGHT as u32,
                Some(self.buffer.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            // Convert RGBA to grayscale luminance and normalize
            let mut frame = Vec::with_capacity(GRID_SIZE);
            for i in 0..GRID_SIZE {
                let idx = i * 4;
                let r = self.buffer[idx] as f32;
                let g = self.buffer[idx + 1] as f32;
                let b = self.buffer[idx + 2] as f32;

                // ITU-R BT.601 luminance
                let luminance = (r * 0.299 + g * 0.587 + b * 0.114) / 255.0;
                frame.push(luminance);
            }

            Ok(frame)
        }
    }

    pub fn capture_frame_with_gate(
        &mut self,
        gate_mask: &[bool; 256],
    ) -> Result<Vec<f32>, String> {
        let mut frame = self.capture_frame()?;

        // Zero out inactive sectors
        let sector_size = 8;
        let sectors_per_row = 16;

        for (sector_idx, &active) in gate_mask.iter().enumerate() {
            if !active {
                let sector_y = sector_idx / sectors_per_row;
                let sector_x = sector_idx % sectors_per_row;
                let y_start = sector_y * sector_size;
                let x_start = sector_x * sector_size;

                for dy in 0..sector_size {
                    for dx in 0..sector_size {
                        let y = y_start + dy;
                        let x = x_start + dx;
                        if y < GRID_HEIGHT && x < GRID_WIDTH {
                            frame[y * GRID_WIDTH + x] = 0.0;
                        }
                    }
                }
            }
        }

        Ok(frame)
    }
}

impl Drop for Win32ScreenCapture {
    fn drop(&mut self) {
        unsafe {
            if let Some(hbitmap) = self.hbitmap {
                let _ = DeleteObject(hbitmap);
            }
            if let Some(hdc_memory) = self.hdc_memory {
                let _ = DeleteDC(hdc_memory);
            }
            if let Some(hdc_screen) = self.hdc_screen {
                let _ = ReleaseDC(HWND::default(), hdc_screen);
            }
        }
    }
}
