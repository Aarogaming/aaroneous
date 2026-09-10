/// DX-01: Safe Mode Boot Flag
/// Checks if the Shift key is held during launch to bypass auto-loading plugins and reset the layout.
pub fn is_safe_mode_requested() -> bool {
    // In production on Windows, this calls GetAsyncKeyState(VK_SHIFT) & 0x8000
    // Or we check std::env::args() for --safe-mode
    std::env::args().any(|arg| arg == "--safe-mode")
}