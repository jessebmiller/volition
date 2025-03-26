use serde::{Deserialize, Serialize};
use super::tools::{ToolCall, ToolFunction};
use serde_json::Value;
use anyhow::{Result, anyhow};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolCallResult {
    pub tool_call_id: String,
    pub output: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Choice {
    pub index: u32,
    pub message: ResponseMessage,
    pub finish_reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse {
    pub id: String,
    pub choices: Vec<Choice>,
}

// Create service-specific deserializers
#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize, Debug)]
struct GeminiCandidate {
    content: GeminiContent,
    #[serde(rename = "finishReason")]
    finish_reason: String,
    #[serde(rename = "functionCall", default)]
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Deserialize, Debug)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
    role: String,
}

#[derive(Deserialize, Debug)]
struct GeminiPart {
    text: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GeminiFunctionCall {
    name: String,
    args: Value,
}

// Parse service-specific response into ApiResponse
pub fn parse_gemini_response(response_json: Value) -> Result<ApiResponse> {
    let gemini_response: GeminiResponse = serde_json::from_value(response_json)?;

    if gemini_response.candidates.is_empty() {
        return Err(anyhow!("No response candidates found"));
    }

    let candidate = &gemini_response.candidates[0];
    let content_text = candidate.content.parts.iter()
        .filter_map(|part| part.text.clone())
        .collect::<Vec<_>>()
        .join("");

    // Convert function call if present
    let tool_calls = if let Some(function_call) = &candidate.function_call {
        Some(vec![ToolCall {
            id: Uuid::new_v4().to_string(), // Generate unique ID
            call_type: "function".to_string(),
            function: ToolFunction {
                name: function_call.name.clone(),
                arguments: function_call.args.to_string(),
            },
        }])
    } else {
        None
    };

    Ok(ApiResponse {
        id: Uuid::new_v4().to_string(), // Generate unique ID
        choices: vec![Choice {
            index: 0,
            message: ResponseMessage {
                role: "assistant".to_string(),
                content: if !content_text.is_empty() { Some(content_text) } else { None },
                tool_calls,
                tool_call_id: None,
            },
            finish_reason: candidate.finish_reason.clone(),
        }],
    })
}
