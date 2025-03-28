// volition-agent-core/src/models/chat.rs
use super::tools::ToolCall;
use serde::{Deserialize, Serialize};

/// Represents a message in the chat history sequence sent to/from the AI.
/// Can represent system, user, assistant, or tool messages.
#[derive(Serialize, Deserialize, Debug, Clone, Default)] // Renamed from ResponseMessage
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_call_id: Option<String>,
}

/// Represents one of the choices returned by the AI API.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage, // Updated field type
    pub finish_reason: String,
}

/// Represents the overall structure of the AI API response.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiResponse {
    pub id: String,
    pub choices: Vec<Choice>,
}

// Commented out unused structs:
// pub struct ToolCallResult { ... }
