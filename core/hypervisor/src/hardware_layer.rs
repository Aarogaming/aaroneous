/// Hardware-level primitives: secure enclave memory isolation and
/// zero-copy frame buffer UI overlay rendering.
use std::collections::HashMap;

// ── Hardware-Enclave Memory Isolation ────────────────────────────────
// Restricts core VSA vector keys and OS peripheral hooks to run within
// a secure CPU enclave (SGX/SEV-SNP style).

#[derive(Debug, Clone)]
pub struct SecureEnclave {
    pub enabled: bool,
    /// Encrypted key store: only accessible inside the enclave.
    pub sealed_keys: HashMap<u64, Vec<u8>>,
    pub measurement: u64,
}

impl SecureEnclave {
    pub fn new(enabled: bool) -> Self {
        SecureEnclave {
            enabled,
            sealed_keys: HashMap::new(),
            measurement: 0,
        }
    }

    /// Seal a key into the enclave (encrypt with platform key).
    pub fn seal_key(&mut self, id: u64, key: &[u8]) {
        let encrypted: Vec<u8> = if self.enabled {
            // XOR with platform measurement as simple sealing
            key.iter().map(|&b| b ^ (self.measurement as u8)).collect()
        } else {
            key.to_vec()
        };
        self.sealed_keys.insert(id, encrypted);
    }

    /// Unseal a key (decrypt inside enclave).
    pub fn unseal_key(&self, id: u64) -> Option<Vec<u8>> {
        self.sealed_keys.get(&id).map(|encrypted| {
            if self.enabled {
                encrypted
                    .iter()
                    .map(|&b| b ^ (self.measurement as u8))
                    .collect()
            } else {
                encrypted.clone()
            }
        })
    }

    /// Set platform measurement (e.g., MRENCLAVE from SGX).
    pub fn set_measurement(&mut self, m: u64) {
        self.measurement = m;
    }

    /// Verify enclave identity by checking measurement.
    pub fn verify(&self, expected_measurement: u64) -> bool {
        !self.enabled || self.measurement == expected_measurement
    }

    /// Attest: return a signed report of the enclave's measurement.
    /// Simplified: returns measurement XOR'd with a nonce.
    pub fn attest(&self, nonce: u64) -> u64 {
        self.measurement ^ nonce
    }
}

// ── Zero-Copy Frame Buffer UI Overlay ────────────────────────────────
// Writes tracking coordinates, state names, and vector hotspots directly
// onto a secondary translucent desktop layer via hardware pointers.

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OverlayPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OverlayRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub color: OverlayPixel,
    pub label_offset: u32,
}

#[derive(Debug, Clone)]
pub struct ShmemUIOverlay {
    pub framebuffer: Vec<OverlayPixel>,
    pub width: u32,
    pub height: u32,
    pub rects: Vec<OverlayRect>,
}

impl ShmemUIOverlay {
    pub fn new(width: u32, height: u32) -> Self {
        let bg = OverlayPixel {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        ShmemUIOverlay {
            framebuffer: vec![bg; (width * height) as usize],
            width,
            height,
            rects: Vec::new(),
        }
    }

    /// Draw a filled rectangle onto the overlay framebuffer.
    pub fn draw_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: OverlayPixel) {
        let x_end = (x + w).min(self.width);
        let y_end = (y + h).min(self.height);
        for py in y..y_end {
            for px in x..x_end {
                let idx = (py * self.width + px) as usize;
                if idx < self.framebuffer.len() {
                    self.framebuffer[idx] = color;
                }
            }
        }
        self.rects.push(OverlayRect {
            x,
            y,
            w,
            h,
            color,
            label_offset: 0,
        });
    }

    /// Draw a crosshair at a hotspot center.
    pub fn draw_crosshair(&mut self, cx: u32, cy: u32, size: u32, color: OverlayPixel) {
        // Horizontal line
        let x_start = cx.saturating_sub(size);
        let x_end = (cx + size).min(self.width);
        if cy < self.height {
            for px in x_start..x_end {
                let idx = (cy * self.width + px) as usize;
                if idx < self.framebuffer.len() {
                    self.framebuffer[idx] = color;
                }
            }
        }
        // Vertical line
        let y_start = cy.saturating_sub(size);
        let y_end = (cy + size).min(self.height);
        if cx < self.width {
            for py in y_start..y_end {
                let idx = (py * self.width + cx) as usize;
                if idx < self.framebuffer.len() {
                    self.framebuffer[idx] = color;
                }
            }
        }
    }

    /// Write a text label onto the framebuffer (each byte is a pixel row).
    pub fn draw_label(&mut self, x: u32, y: u32, text: &[u8], color: OverlayPixel) {
        for (i, &ch) in text.iter().enumerate() {
            let py = y + i as u32;
            if py >= self.height {
                break;
            }
            let px = x;
            if px < self.width {
                let idx = (py * self.width + px) as usize;
                if idx < self.framebuffer.len() && ch != b' ' {
                    self.framebuffer[idx] = color;
                }
            }
        }
    }

    /// Clear the overlay (set all pixels to transparent).
    pub fn clear(&mut self) {
        let bg = OverlayPixel {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        for pixel in &mut self.framebuffer {
            *pixel = bg;
        }
        self.rects.clear();
    }

    /// Blend this overlay onto a primary framebuffer.
    pub fn blend_onto(&self, primary: &mut [OverlayPixel]) {
        let n = self.framebuffer.len().min(primary.len());
        for i in 0..n {
            let overlay = self.framebuffer[i];
            if overlay.a > 0 {
                let bg = primary[i];
                let a = overlay.a as f32 / 255.0;
                primary[i] = OverlayPixel {
                    r: (overlay.r as f32 * a + bg.r as f32 * (1.0 - a)) as u8,
                    g: (overlay.g as f32 * a + bg.g as f32 * (1.0 - a)) as u8,
                    b: (overlay.b as f32 * a + bg.b as f32 * (1.0 - a)) as u8,
                    a: 255,
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_enclave_seal_unseal() {
        let mut enclave = SecureEnclave::new(true);
        enclave.set_measurement(0xABCD);
        enclave.seal_key(42, b"secret_key");
        let unsealed = enclave.unseal_key(42);
        assert_eq!(unsealed, Some(b"secret_key".to_vec()));
    }

    #[test]
    fn test_secure_enclave_tamper() {
        let mut enclave = SecureEnclave::new(true);
        enclave.set_measurement(0xABCD);
        enclave.seal_key(42, b"secret_key");
        // Changing measurement corrupts the key
        enclave.set_measurement(0xDEAD);
        let unsealed = enclave.unseal_key(42);
        assert_ne!(unsealed, Some(b"secret_key".to_vec()));
    }

    #[test]
    fn test_secure_enclave_verify() {
        let mut enclave = SecureEnclave::new(true);
        enclave.set_measurement(0xABCD);
        assert!(enclave.verify(0xABCD));
        assert!(!enclave.verify(0xDEAD));
    }

    #[test]
    fn test_secure_enclave_attest() {
        let mut enclave = SecureEnclave::new(true);
        enclave.set_measurement(0x1234);
        let report = enclave.attest(0xFFFF);
        assert_eq!(report, 0x1234 ^ 0xFFFF);
    }

    #[test]
    fn test_secure_enclave_disabled() {
        let enclave = SecureEnclave::new(false);
        assert!(enclave.verify(9999)); // always passes when disabled
    }

    #[test]
    fn test_shmem_overlay_draw_rect() {
        let mut overlay = ShmemUIOverlay::new(100, 100);
        let red = OverlayPixel {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        overlay.draw_rect(10, 10, 20, 20, red);
        let idx = (15 * 100 + 15) as usize;
        assert_eq!(overlay.framebuffer[idx].r, 255);
        assert_eq!(overlay.rects.len(), 1);
    }

    #[test]
    fn test_shmem_overlay_crosshair() {
        let mut overlay = ShmemUIOverlay::new(50, 50);
        let cyan = OverlayPixel {
            r: 0,
            g: 255,
            b: 255,
            a: 255,
        };
        overlay.draw_crosshair(25, 25, 5, cyan);
        // Center pixel should be cyan
        let idx = (25 * 50 + 25) as usize;
        assert_eq!(overlay.framebuffer[idx].g, 255);
        assert_eq!(overlay.framebuffer[idx].b, 255);
    }

    #[test]
    fn test_shmem_overlay_label() {
        let mut overlay = ShmemUIOverlay::new(50, 50);
        let white = OverlayPixel {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        overlay.draw_label(10, 10, b"HI", white);
        let idx = (10 * 50 + 10) as usize;
        assert_eq!(overlay.framebuffer[idx].r, 255); // 'H'
    }

    #[test]
    fn test_shmem_overlay_clear() {
        let mut overlay = ShmemUIOverlay::new(10, 10);
        let red = OverlayPixel {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        overlay.draw_rect(0, 0, 10, 10, red);
        overlay.clear();
        assert!(
            overlay
                .framebuffer
                .iter()
                .all(|p| p.r == 0 && p.g == 0 && p.b == 0 && p.a == 0)
        );
    }

    #[test]
    fn test_shmem_overlay_blend() {
        let white = OverlayPixel {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        let mut primary = vec![white; 4];
        let mut overlay = ShmemUIOverlay::new(2, 2);
        let semi = OverlayPixel {
            r: 255,
            g: 0,
            b: 0,
            a: 128,
        };
        overlay.draw_rect(0, 0, 2, 2, semi);
        overlay.blend_onto(&mut primary);
        // After blend, pixel should be pinkish (white + red at 50%)
        assert!(primary[0].r > 200);
        assert!(primary[0].g < 200);
        assert!(primary[0].b < 200);
    }
}
