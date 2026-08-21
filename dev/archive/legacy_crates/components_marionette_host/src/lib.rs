use anyhow::Result;
use enigo::{Enigo, Settings};
use scrap::{Capturer, Display};
use std::io::ErrorKind::WouldBlock;
use std::thread;
use std::time::Duration;

pub struct MarionetteHost {
    enigo: Enigo,
    permission_level: String,
}

impl MarionetteHost {
    pub fn new(permission_level: &str) -> Self {
        let settings = Settings::default();
        MarionetteHost {
            enigo: Enigo::new(&settings).expect("Failed to initialize enigo"),
            permission_level: permission_level.to_string(),
        }
    }

    pub fn pull_string_mouse(&mut self, _x: i32, _y: i32) -> Result<String, String> {
        if self.permission_level == "trusted" {
            // TODO: Fix enigo API usage
            Ok(format!("Mouse moved (mocked)"))
        } else {
            Err("Permission denied: mouse control requires trusted code".to_string())
        }
    }

    pub fn pull_string_vision(&self) -> Result<String, String> {
        if self.permission_level == "trusted" {
            let display = Display::primary().map_err(|e| e.to_string())?;
            let mut capturer = Capturer::new(display).map_err(|e| e.to_string())?;

            let w = capturer.width();
            let h = capturer.height();

            // Loop until a frame is captured
            let _frame = loop {
                match capturer.frame() {
                    Ok(f) => break f,
                    Err(ref e) if e.kind() == WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => return Err(e.to_string()),
                }
            };

            Ok(format!("Screenshot captured: {}x{}", w, h))
        } else {
            Err("Permission denied: vision access requires trusted code".to_string())
        }
    }

    pub fn is_trusted(&self) -> bool {
        self.permission_level == "trusted"
    }
}

impl Default for MarionetteHost {
    fn default() -> Self {
        Self::new("trusted")
    }
}
