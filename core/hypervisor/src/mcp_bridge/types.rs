use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MpcCapability {
    pub domain: String,
    pub method_name: String,
    pub version: String,
    pub schema: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MpcMessage {
    pub trace_id: String,
    pub timestamp: i64,
    pub method: String,
    pub params: HashMap<String, Value>,
    pub reply_to: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MpcResponse {
    pub trace_id: String,
    pub status: String,
    pub result: Value,
}
