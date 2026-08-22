// HID Output Bridge - Converts motor intents to Win32 SendInput hardware events

use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT,
    SendInput,
};

/// Motor intent from the reflex kernel
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MotorIntent {
    pub delta_x: f32,
    pub delta_y: f32,
    pub binary_action_register: u64,
}

// Action register bit flags
pub const ACTION_MOUSE_MOVE: u64 = 1 << 0;
pub const ACTION_MOUSE_LEFT_DOWN: u64 = 1 << 1;
pub const ACTION_MOUSE_LEFT_UP: u64 = 1 << 2;
pub const ACTION_MOUSE_RIGHT_DOWN: u64 = 1 << 3;
pub const ACTION_MOUSE_RIGHT_UP: u64 = 1 << 4;
pub const ACTION_MOUSE_WHEEL: u64 = 1 << 5;
pub const ACTION_KEY_PRESS: u64 = 1 << 6;
pub const ACTION_KEY_RELEASE: u64 = 1 << 7;
pub const ACTION_CLICK: u64 = 1 << 8;
pub const ACTION_DOUBLE_CLICK: u64 = 1 << 9;
pub const ACTION_DRAG_START: u64 = 1 << 10;
pub const ACTION_DRAG_END: u64 = 1 << 11;

/// Converts motor intents to Win32 SendInput hardware events
pub struct HIDOutputBridge {
    mouse_sensitivity: f32,
}

impl Default for HIDOutputBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl HIDOutputBridge {
    pub fn new() -> Self {
        Self {
            mouse_sensitivity: 1.0,
        }
    }

    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.mouse_sensitivity = sensitivity;
        self
    }

    pub fn execute_intent(&self, intent: &MotorIntent) {
        // Keep this low-level bridge fail-closed as well as the higher-level
        // Marionette host. Direct callers must explicitly opt into live host
        // input through the same runtime safety permit.
        if std::env::var("AARONEOUS_ALLOW_HOST_INPUT").as_deref() != Ok("1") {
            return;
        }

        let actions = intent.binary_action_register;

        if actions & ACTION_MOUSE_MOVE != 0 {
            self.move_mouse(intent.delta_x, intent.delta_y);
        }

        if actions & ACTION_MOUSE_LEFT_DOWN != 0 {
            self.mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0);
        }

        if actions & ACTION_MOUSE_LEFT_UP != 0 {
            self.mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0);
        }

        if actions & ACTION_MOUSE_RIGHT_DOWN != 0 {
            self.mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0);
        }

        if actions & ACTION_MOUSE_RIGHT_UP != 0 {
            self.mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0, 0);
        }

        if actions & ACTION_MOUSE_WHEEL != 0 {
            let wheel_delta = (intent.delta_y * 120.0) as i32;
            self.mouse_event(MOUSEEVENTF_WHEEL, 0, 0, wheel_delta as u32);
        }

        if actions & ACTION_CLICK != 0 {
            self.click();
        }

        if actions & ACTION_DOUBLE_CLICK != 0 {
            self.double_click();
        }

        if actions & ACTION_DRAG_START != 0 {
            self.mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0);
        }

        if actions & ACTION_DRAG_END != 0 {
            self.mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0);
        }
    }

    fn move_mouse(&self, delta_x: f32, delta_y: f32) {
        let dx = (delta_x * self.mouse_sensitivity) as i32;
        let dy = (delta_y * self.mouse_sensitivity) as i32;

        if dx == 0 && dy == 0 {
            return;
        }

        self.mouse_event(MOUSEEVENTF_MOVE, dx, dy, 0);
    }

    fn mouse_event(&self, flags: MOUSE_EVENT_FLAGS, dx: i32, dy: i32, mouse_data: u32) {
        unsafe {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx,
                        dy,
                        mouseData: mouse_data,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn click(&self) {
        self.mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0);
        std::thread::sleep(std::time::Duration::from_millis(10));
        self.mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0);
    }

    fn double_click(&self) {
        self.click();
        std::thread::sleep(std::time::Duration::from_millis(50));
        self.click();
    }
}
