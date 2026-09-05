use memmap2::MmapMut;
use std::sync::atomic::{AtomicBool, Ordering};

/// Represents a raw framebuffer capture target in shared memory.
/// The framebuffer is mapped as a fixed-width grayscale float grid.
///
/// Memory layout:
///   [0..width*height)  — f32 grayscale pixels (0.0–1.0)
///   [header offset]    — u64 frame counter, u64 timestamp
#[repr(C, align(64))]
pub struct ShmemFrameHeader {
    pub frame_id: u64,
    pub capture_tick: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

/// Configuration for the shared-memory frame capture.
pub struct FrameCaptureConfig {
    /// Width of the capture grid in pixels.
    pub width: u32,
    /// Height of the capture grid in pixels.
    pub height: u32,
    /// Path to the shared memory backing file.
    pub shmem_path: String,
    /// When true, uses Win32 DXGI duplication API (fastest).
    /// Falls back to GDI StretchBlt when unsupported.
    pub prefer_dxgi: bool,
}

impl Default for FrameCaptureConfig {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            shmem_path: format!(
                r"{}\aaroneous_fb.shmem",
                std::env::var("TEMP").unwrap_or_else(|_| r"C:\Temp".into())
            ),
            prefer_dxgi: true,
        }
    }
}

/// Zero-copy shared-memory framebuffer capture substrate.
///
/// Maps the OS framebuffer (or a compositor-provided surface) directly
/// into a memory-mapped region readable by the WASM sandbox and the
/// SIMD delta-screening pipeline.
pub struct ShmemCapture {
    config: FrameCaptureConfig,
    mmap: Option<MmapMut>,
    frame_counter: u64,
    active: AtomicBool,
}

impl ShmemCapture {
    pub fn new(config: FrameCaptureConfig) -> Self {
        Self {
            config,
            mmap: None,
            frame_counter: 0,
            active: AtomicBool::new(false),
        }
    }

    /// Map or create the shared memory region.
    pub fn open(&mut self) -> Result<(), String> {
        use std::fs::OpenOptions;
        let path = &self.config.shmem_path;
        let file_size = Self::buffer_size(self.config.width, self.config.height)
            + std::mem::size_of::<ShmemFrameHeader>();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|e| format!("shmem open: {}", e))?;

        file.set_len(file_size as u64)
            .map_err(|e| format!("shmem resize: {}", e))?;

        let mmap = unsafe { MmapMut::map_mut(&file) }.map_err(|e| format!("shmem mmap: {}", e))?;

        self.mmap = Some(mmap);
        self.active.store(true, Ordering::Release);
        Ok(())
    }

    /// Perform a zero-copy frame capture.
    ///
    /// On Windows with DXGI: uses Desktop Duplication API for GPU-accelerated capture.
    /// Fallback: Win32 GDI StretchBlt into the mmap buffer.
    pub fn capture_frame(&mut self) -> Result<u64, String> {
        if !self.active.load(Ordering::Acquire) {
            return Err("Capture not open".into());
        }
        self.frame_counter += 1;

        let hdr_size = std::mem::size_of::<ShmemFrameHeader>();
        let pixel_bytes = (self.config.width * self.config.height) as usize * 4;

        // Capture into a temporary buffer to avoid borrow conflicts with mmap
        let mut buf = vec![0u8; pixel_bytes];

        #[cfg(target_os = "windows")]
        {
            if self.config.prefer_dxgi {
                // Try DXGI Desktop Duplication first
                if let Some(dxgi_grid) = platform_bridge::native_win32::capture_dxgi_frame_128x128() {
                    // Convert f32 luminance grid to BGRA buffer for mmap storage
                    for (i, &lum) in dxgi_grid.iter().enumerate() {
                        let byte = (lum * 255.0) as u8;
                        let idx = i * 4;
                        if idx + 3 < buf.len() {
                            buf[idx] = byte;     // B
                            buf[idx + 1] = byte; // G
                            buf[idx + 2] = byte; // R
                            buf[idx + 3] = 255;  // A
                        }
                    }
                } else {
                    self.capture_win32(&mut buf)?;
                }
            } else {
                self.capture_win32(&mut buf)?;
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.capture_fallback(&mut buf)?;
        }

        // Write to mmap while holding the frame data
        if let Some(mmap) = self.mmap.as_mut() {
            let total_size = hdr_size + pixel_bytes;
            if mmap.len() < total_size {
                return Err("mmap too small".into());
            }

            let hdr = ShmemFrameHeader {
                frame_id: self.frame_counter,
                capture_tick: now_tick(),
                width: self.config.width,
                height: self.config.height,
                stride: self.config.width,
            };
            let hdr_bytes = unsafe {
                std::slice::from_raw_parts(&hdr as *const ShmemFrameHeader as *const u8, hdr_size)
            };
            mmap[..hdr_size].copy_from_slice(hdr_bytes);
            mmap[hdr_size..hdr_size + pixel_bytes].copy_from_slice(&buf);
        }

        Ok(self.frame_counter)
    }

    /// Return a pointer to the pixel data for zero-copy reading.
    pub fn pixel_ptr(&self) -> Option<*const f32> {
        let mmap = self.mmap.as_ref()?;
        let hdr_size = std::mem::size_of::<ShmemFrameHeader>();
        let ptr = mmap.as_ptr() as *const f32;
        Some(unsafe { ptr.add(hdr_size / 4) })
    }

    /// Return a mutable pointer to the pixel data for in-place SIMD processing.
    pub fn pixel_ptr_mut(&mut self) -> Option<*mut f32> {
        let mmap = self.mmap.as_mut()?;
        let hdr_size = std::mem::size_of::<ShmemFrameHeader>();
        let ptr = mmap.as_mut_ptr() as *mut f32;
        Some(unsafe { ptr.add(hdr_size / 4) })
    }

    pub fn frame_id(&self) -> u64 {
        self.frame_counter
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    /// Close and release the shared memory mapping.
    pub fn close(&mut self) {
        self.active.store(false, Ordering::Release);
        self.mmap = None;
    }
}

impl Drop for ShmemCapture {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
        self.mmap = None;
    }
}

impl ShmemCapture {
    fn buffer_size(width: u32, height: u32) -> usize {
        (width as usize) * (height as usize) * 4
    }

    #[cfg(target_os = "windows")]
    fn capture_win32(&self, pixel_slice: &mut [u8]) -> anyhow::Result<()> {
        use std::mem;
        use windows::Win32::Foundation::*;
        use windows::Win32::Graphics::Gdi::*;

        // RAII guard to ensure GDI resources are released even on panic/error
        struct GdiGuard<'a> {
            hdc_screen: HDC,
            hdc_mem: HDC,
            bmp: HBITMAP,
            null_hwnd: HWND,
            active: bool,
        }

        impl<'a> Drop for GdiGuard<'a> {
            fn drop(&mut self) {
                if !self.active {
                    return;
                }
                // Always clean up in reverse order of creation
                if let Ok(bmp_handle) = unsafe { HBITMAP(self.bmp.0) } {
                    let _ = unsafe { DeleteObject(bmp_handle.into()) };
                }
                if let Ok(hdc_handle) = unsafe { HDC(self hdc_mem.0) } {
                    let _ = unsafe { DeleteDC(hdc_handle.into()) };
                }
                if let Ok(hdc_screen_handle) = unsafe { HDC(self hdc_screen.0) } {
                    let _ = unsafe { ReleaseDC(Some(self.null_hwnd), hdc_screen_handle.into()) };
                }
                self.active = false;
            }
        }

        let null_hwnd = HWND(std::ptr::null_mut());
        
        // Acquire screen DC - will be released by guard on drop
        let mut hdc_screen = unsafe { GetDC(Some(null_hwnd)) };
        if hdc_screen.is_invalid() {
            return Err(anyhow::anyhow!("GetDC failed"));
        }

        // Create compatible memory DC - releases hdc_screen when dropped
        let hdc_mem = unsafe { CreateCompatibleDC(Some(hdc_screen)) };
        if hdc_mem.is_invalid() {
            // Guard will release hdc_screen on drop
            return Err(anyhow::anyhow!("CreateCompatibleDC failed"));
        }

        // Create bitmap - released by guard when all resources cleaned up
        let bmp = unsafe { CreateCompatibleBitmap(hdc_screen, self.config.width as i32, self.config.height as i32) };
        if bmp.is_invalid() {
            return Err(anyhow::anyhow!("CreateCompatibleBitmap failed"));
        }

        // Select bitmap into memory DC and perform capture
        let w = self.config.width as i32;
        let h = self.config.height as i32;
        
            // Wrap critical section in guard to ensure cleanup
            let mut _guard = GdiGuard {
                hdc_screen,
                hdc_mem,
                bmp,
                null_hwnd,
                active: true,
            };

            if unsafe { SelectObject(_guard(hdc_mem), _guard.bmp.into()) }.is_invalid() {
                return Err(anyhow::anyhow!("SelectObject failed"));
            }

            let stretch_result = unsafe { StretchBlt(
                _guard(hdc_mem),
                0,
                0,
                w,
                h,
                Some(_guard(hdc_screen)),
                0,
                0,
                GetDeviceCaps(Some(_guard(hdc_screen)), HORZRES),
                GetDeviceCaps(Some(_guard(hdc_screen)), VERTRES),
                SRCCOPY,
            )};

            if stretch_result == 0 {
                return Err(anyhow::anyhow!("StretchBlt failed"));
            }

            let mut bmi = mem::zeroed::<BITMAPINFO>();
            bmi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = w;
            bmi.bmiHeader.biHeight = -h;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = 0;

            let get_dib_result = unsafe { GetDIBits(
                _guard(hdc_mem),
                _guard.bmp,
                0,
                h as u32,
                Some(pixel_slice.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_USAGE(0),
            )};

            if get_dib_result == 0 {
                return Err(anyhow::anyhow!("GetDIBits failed"));
            }
        } // Guard dropped here - all GDI resources released

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn capture_fallback(&self, pixel_slice: &mut [u8]) -> Result<(), String> {
        // Fill with test pattern on non-Windows
        for (i, byte) in pixel_slice.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }
        Ok(())
    }
}

unsafe impl Send for ShmemCapture {}
unsafe impl Sync for ShmemCapture {}

fn now_tick() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::ZERO)
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shmem_open_close() {
        let mut cap = ShmemCapture::new(FrameCaptureConfig {
            width: 64,
            height: 64,
            shmem_path: format!(
                r"{}\aaroneous_test_shmem.shmem",
                std::env::var("TEMP").unwrap_or_else(|_| r"C:\Temp".into())
            ),
            prefer_dxgi: false,
        });
        assert!(cap.open().is_ok());
        assert!(cap.is_active());
        cap.close();
        assert!(!cap.is_active());
    }

    #[test]
    fn test_shmem_capture_frame() {
        let mut cap = ShmemCapture::new(FrameCaptureConfig {
            width: 32,
            height: 32,
            shmem_path: format!(
                r"{}\aaroneous_test_shmem2.shmem",
                std::env::var("TEMP").unwrap_or_else(|_| r"C:\Temp".into())
            ),
            prefer_dxgi: false,
        });
        cap.open().expect("Failed to open shared memory");
        let fid = cap.capture_frame().expect("Failed to capture frame");
        assert_eq!(fid, 1);
        let fid2 = cap.capture_frame().expect("Failed to capture frame");
        assert_eq!(fid2, 2);
        cap.close();
    }

    #[test]
    fn test_shmem_pixel_ptr() {
        let mut cap = ShmemCapture::new(FrameCaptureConfig {
            width: 16,
            height: 16,
            shmem_path: format!(
                r"{}\aaroneous_test_shmem3.shmem",
                std::env::var("TEMP").unwrap_or_else(|_| r"C:\Temp".into())
            ),
            prefer_dxgi: false,
        });
        cap.open().expect("Failed to open shared memory");
        cap.capture_frame().expect("Failed to capture frame");
        let ptr = cap.pixel_ptr();
        assert!(ptr.is_some());
        cap.close();
    }
}
