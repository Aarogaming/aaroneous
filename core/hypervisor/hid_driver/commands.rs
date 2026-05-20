/// HID Commands and Responses
///
/// Defines the command types that can be sent to the HID driver
/// and the corresponding responses.

use serde::{Deserialize, Serialize};

/// Command to send to HID driver
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HidCommand {
    /// Move mouse cursor to (x, y)
    MouseMove { x: i32, y: i32 },
    
    /// Click mouse button at (x, y)
    MouseClick { button: MouseButton, x: i32, y: i32 },
    
    /// Release mouse button
    MouseRelease { button: MouseButton },
    
    /// Press keyboard key
    KeyPress { key: u32, modifiers: u8 },
    
    /// Release keyboard key
    KeyRelease { key: u32 },
    
    /// Scroll (positive = up, negative = down)
    Scroll { delta: i32 },
    
    /// Get current cursor position
    GetCursorPos,
    
    /// Query key state (is it pressed?)
    QueryKeyState { key: u32 },
}

/// Response from HID driver
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HidResponse {
    /// Command executed successfully
    Success,
    
    /// Current cursor position
    CursorPos { x: i32, y: i32 },
    
    /// Key state query result
    KeyState { pressed: bool },
    
    /// Error occurred
    Error { reason: String },
}

/// Mouse button type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
}

// Keyboard modifier constants
pub mod modifiers {
    pub const SHIFT: u8 = 0x01;
    pub const CTRL: u8 = 0x02;
    pub const ALT: u8 = 0x04;
    pub const WIN: u8 = 0x08;
}

// Virtual key codes (Windows VK_ constants)
pub mod keycodes {
    pub const VK_LBUTTON: u32 = 0x01;
    pub const VK_RBUTTON: u32 = 0x02;
    pub const VK_CANCEL: u32 = 0x03;
    pub const VK_MBUTTON: u32 = 0x04;
    
    // Letters
    pub const VK_A: u32 = 0x41;
    pub const VK_B: u32 = 0x42;
    pub const VK_C: u32 = 0x43;
    pub const VK_D: u32 = 0x44;
    pub const VK_E: u32 = 0x45;
    pub const VK_F: u32 = 0x46;
    pub const VK_G: u32 = 0x47;
    pub const VK_H: u32 = 0x48;
    pub const VK_I: u32 = 0x49;
    pub const VK_J: u32 = 0x4A;
    pub const VK_K: u32 = 0x4B;
    pub const VK_L: u32 = 0x4C;
    pub const VK_M: u32 = 0x4D;
    pub const VK_N: u32 = 0x4E;
    pub const VK_O: u32 = 0x4F;
    pub const VK_P: u32 = 0x50;
    pub const VK_Q: u32 = 0x51;
    pub const VK_R: u32 = 0x52;
    pub const VK_S: u32 = 0x53;
    pub const VK_T: u32 = 0x54;
    pub const VK_U: u32 = 0x55;
    pub const VK_V: u32 = 0x56;
    pub const VK_W: u32 = 0x57;
    pub const VK_X: u32 = 0x58;
    pub const VK_Y: u32 = 0x59;
    pub const VK_Z: u32 = 0x5A;
    
    // Numbers
    pub const VK_0: u32 = 0x30;
    pub const VK_1: u32 = 0x31;
    pub const VK_2: u32 = 0x32;
    pub const VK_3: u32 = 0x33;
    pub const VK_4: u32 = 0x34;
    pub const VK_5: u32 = 0x35;
    pub const VK_6: u32 = 0x36;
    pub const VK_7: u32 = 0x37;
    pub const VK_8: u32 = 0x38;
    pub const VK_9: u32 = 0x39;
    
    // Function keys
    pub const VK_F1: u32 = 0x70;
    pub const VK_F2: u32 = 0x71;
    pub const VK_F3: u32 = 0x72;
    pub const VK_F4: u32 = 0x73;
    pub const VK_F5: u32 = 0x74;
    pub const VK_F6: u32 = 0x75;
    pub const VK_F7: u32 = 0x76;
    pub const VK_F8: u32 = 0x77;
    pub const VK_F9: u32 = 0x78;
    pub const VK_F10: u32 = 0x79;
    pub const VK_F11: u32 = 0x7A;
    pub const VK_F12: u32 = 0x7B;
    
    // Special keys
    pub const VK_ESCAPE: u32 = 0x1B;
    pub const VK_TAB: u32 = 0x09;
    pub const VK_SPACE: u32 = 0x20;
    pub const VK_RETURN: u32 = 0x0D;
    pub const VK_SHIFT: u32 = 0x10;
    pub const VK_CONTROL: u32 = 0x11;
    pub const VK_ALT: u32 = 0x12;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mouse_button_values() {
        assert_eq!(MouseButton::Left as u32, 0);
        assert_eq!(MouseButton::Right as u32, 1);
        assert_eq!(MouseButton::Middle as u32, 2);
    }
    
    #[test]
    fn test_keycode_a() {
        assert_eq!(keycodes::VK_A, 0x41);
    }
    
    #[test]
    fn test_modifier_values() {
        assert_eq!(modifiers::SHIFT, 0x01);
        assert_eq!(modifiers::CTRL, 0x02);
        assert_eq!(modifiers::ALT, 0x04);
    }
    
    #[test]
    fn test_hid_command_serialization() {
        let cmd = HidCommand::MouseMove { x: 100, y: 200 };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: HidCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }
}
