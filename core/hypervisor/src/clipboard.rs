use arboard::Clipboard;
use parking_lot::Mutex;
use std::sync::OnceLock;

static CLIPBOARD: OnceLock<Mutex<Option<Clipboard>>> = OnceLock::new();

fn get_clipboard() -> &'static Mutex<Option<Clipboard>> {
    CLIPBOARD.get_or_init(|| Mutex::new(Clipboard::new().ok()))
}

/// Safely read the current text from the global OS clipboard.
pub fn read_text() -> Option<String> {
    let mut cb_lock = get_clipboard().lock();
    if let Some(cb) = cb_lock.as_mut() {
        return cb.get_text().ok();
    }
    None
}

/// Safely write text to the global OS clipboard.
pub fn write_text(text: &str) -> bool {
    let mut cb_lock = get_clipboard().lock();
    if let Some(cb) = cb_lock.as_mut() {
        return cb.set_text(text).is_ok();
    }
    false
}