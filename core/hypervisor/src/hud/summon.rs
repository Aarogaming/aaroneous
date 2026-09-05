use std::thread;
use eframe::egui::{self, ViewportCommand};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, MOD_CONTROL, MOD_ALT, VIRTUAL_KEY};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

/// Spawns a background thread that registers a Win32 Global Hotkey (Ctrl + Alt + Space).
/// When pressed, it uses the provided egui::Context to force the application to the foreground.
pub fn spawn_global_summon_hook(ctx: egui::Context) {
    thread::spawn(move || unsafe {
        // 1 is the hotkey ID. 0x20 is VK_SPACE.
        let success = RegisterHotKey(None, 1, MOD_CONTROL | MOD_ALT, 0x20);
        
        if success.is_ok() {
            log::info!("Registered global summon hotkey: Ctrl + Alt + Space");
            let mut msg = MSG::default();
            
            // This loop puts the thread to sleep until a Win32 message arrives. 0% CPU usage.
            while GetMessageW(&mut msg, None, 0, 0).into() {
                if msg.message == WM_HOTKEY {
                    log::info!("Global hotkey triggered, summoning HUD to foreground...");
                    
                    // Tell egui to unminimize, show, and focus the window
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                    
                    // Request a repaint so the UI thread wakes up instantly
                    ctx.request_repaint();
                }
            }
        } else {
            log::error!("Failed to register global hotkey (Ctrl + Alt + Space is likely in use)");
        }
    });
}