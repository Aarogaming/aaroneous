use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarionetteAction {
    MouseMove { x: i32, y: i32 },
    MouseButtonClick { button: String },
    KeyPress { key: String },
    WindowFocus { window_id: String },
    EBusEvent { event_id: String, payload: Vec<u8> },
}
