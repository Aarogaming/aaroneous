use anyhow::{Result, anyhow};
use enigo::{Enigo, MouseControllable, KeyboardControllable};
use scrap::{Capturer, Display};
use std::io::ErrorKind::WouldBlock;
use std::thread;
use std::time::Duration;

pub struct MarionetteHost {
    enigo: Enigo,
    permission_level: PermissionLevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PermissionLevel {
    Trusted,
    Untrusted,
}

impl MarionetteHost {
    pub fn new(permission_level: PermissionLevel) -> Self {
        MarionetteHost {
            enigo: Enigo::new(),
            permission_level,
        }
    }

    pub fn pull_string_mouse(&mut self, x: i32, y: i32) -> Result<String, String> {
        if self.permission_level == PermissionLevel::Trusted {
            self.enigo.mouse_move_to(x, y);
            Ok(format!("Mouse: x={}, y={}", x, y))
        } else {
            Err("Permission denied: mouse control requires trusted code".to_string())
        }
    }

    pub fn pull_string_vision(&self) -> Result<String, String> {
        if self.permission_level == PermissionLevel::Trusted {
            let display = Display::primary().map_err(|e| e.to_string())?;
            let mut capturer = Capturer::new(display).map_err(|e| e.to_string())?;
            
            let w = capturer.width();
            let h = capturer.height();
            
            // Loop until a frame is captured
            let frame = loop {
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

    pub fn set_permission_level(&mut self, level: PermissionLevel) {
        self.permission_level = level;
    }

    pub fn is_trusted(&self) -> bool {
        self.permission_level == PermissionLevel::Trusted
    }
}

impl Default for MarionetteHost {
    fn default() -> Self {
        Self::new(PermissionLevel::Untrusted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_control_untrusted() {
        let mut host = MarionetteHost::new(PermissionLevel::Untrusted);
        let result = host.pull_string_mouse(100, 200);
        assert!(result.is_err());
    }

    #[test]
    fn test_mouse_control_trusted() {
        let mut host = MarionetteHost::new(PermissionLevel::Trusted);
        // This might actually move the mouse if run locally, be careful in CI
        let result = host.pull_string_mouse(100, 200);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Mouse: x=100, y=200");
    }
}
