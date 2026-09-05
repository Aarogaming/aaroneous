// dev/tools/afc/src/router/mod.rs
pub mod client;
pub mod extractor;
pub mod tools;
pub mod types;

pub use client::TypedRouterClient;
pub use extractor::TypedExtractor;
pub use tools::{AuditDefectItem, PatchProposal, ToolRegistry};
pub use types::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Choice, FunctionDefinition,
    ResponseMessage, ToolCall, ToolDefinition, UsageInfo,
};
