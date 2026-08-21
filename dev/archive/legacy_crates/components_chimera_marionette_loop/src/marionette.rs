use anyhow::{Result, Context};
use async_trait::async_trait;
use enigo::{Enigo, Settings, Button, Key};
use scrap::{Capturer, Display};
use std::io::ErrorKind::WouldBlock;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct VisualObservation {
    pub screen_buffer: Vec<u8>,
    pub timestamp: u128,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone)]
pub struct HidCommand {
    pub action: String,
    pub x: i32,
    pub y: i32,
}

#[async_trait]
pub trait MarionetteHost: Send + Sync {
    /// Ingest visual state
    async fn pull_visual_perception(&self) -> Result<VisualObservation>;
    
    /// Inject hardware events (mouse/keyboard)
    async fn inject_hid_event(&mut self, event: HidCommand) -> Result<()>;
}

pub struct NativeMarionette {
    enigo: Enigo,
}

impl NativeMarionette {
    pub fn new() -> Self {
        let settings = Settings::default();
        Self {
            enigo: Enigo::new(&settings).expect("Failed to initialize enigo"),
        }
    }
}

#[async_trait]
impl MarionetteHost for NativeMarionette {
    async fn pull_visual_perception(&self) -> Result<VisualObservation> {
        let display = Display::primary().context("Failed to get primary display")?;
        let mut capturer = Capturer::new(display).context("Failed to initialize capturer")?;
        
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
                Err(e) => return Err(e).context("Failed to capture frame"),
            }
        };
        
        Ok(VisualObservation {
            screen_buffer: frame.to_vec(),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis(),
            width: w,
            height: h,
        })
    }
    
    async fn inject_hid_event(&mut self, event: HidCommand) -> Result<()> {
        match event.action.as_str() {
            "mouse_move" => {
                self.enigo.mouse_move(event.x, event.y);
            },
            "mouse_click" => {
                self.enigo.mouse_click(Button::Left);
            },
            "key_press" => {
                if let Some(c) = std::char::from_u32(event.x as u32) {
                    self.enigo.key_click(Key::Layout(c));
                }
            },
            _ => tracing::warn!(target: "marionette_host", "Unknown HID action: {}", event.action),
        }
        Ok(())
    }
}
