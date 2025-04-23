use super::ChatApiProvider;
use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::models::tools::{ToolCall, ToolDefinition};
use anyhow::{Result, anyhow, Context};
use serde_json::{json, Value};
use std::collections::HashMap;
use toml::Value as TomlValue;
use tracing::{error, warn};

pub struct GeminiProvider;

impl GeminiProvider {
    pub fn new() -> Self {
        Self
    }
}

impl ChatApiProvider for GeminiProvider {
    fn build_payload(
        &self,
        model_name: &str,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
        parameters: Option<&TomlValue>,
    ) -> Result<Value> {
        let mut gemini_payload = json!({
            "model": model_name,
            "contents": []
        });

        // Add system instructions
        let mut system_instruction_parts = Vec::new();
        for message in &messages {
            if message.role == "system" {
                if let Some(content) = &message.content {
                    system_instruction_parts.push(json!({ "text": content }));
                }
            }
        }

        // Add conversation messages
        let mut gemini_contents = Vec::new();
        for message in messages {
            if let Some(role) = map_role_to_gemini(&message.role) {
                match role {
                    "user" | "model" => {
                        if let Some(content) = message.content {
                            gemini_contents.push(json!({
                                "role": role,
                                "parts": [{"text": content}]
                            }));
                        }
                    }
                    "function" => {
                        if let Some(tool_call_id) = message.tool_call_id {
                            let response_content = message.content.unwrap_or_default();
                            let response_json: Value = serde_json::from_str(&response_content)
                                .unwrap_or_else(|_| json!(response_content));
                            gemini_contents.push(json!({
                                "role": role,
                                "parts": [{
                                    "functionResponse": {
                                        "name": tool_call_id,
                                        "response": {"content": response_json}
                                    }
                                }]
                            }));
                        }
                    }
                    _ => warn!("Unexpected role in Gemini message: {}", role),
                }
            }
        }

        // Add tools if present
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let tools_with_type: Vec<Value> = tools
                    .iter()
                    .map(|t| {
                        json!({
                            "type": "function",
                            "function": {
                                "name": t.name,
                                "description": t.description,
                                "parameters": t.parameters
                            }
                        })
                    })
                    .collect();
                gemini_payload["tools"] = json!(tools_with_type);
            }
        }

        // Add parameters if present
        if let Some(params) = parameters {
            if let Some(temperature) = params.get("temperature").and_then(|t| t.as_float()) {
                gemini_payload["temperature"] = json!(temperature);
            }
        }

        Ok(gemini_payload)
    }

    fn parse_response(&self, response_body: &str) -> Result<ApiResponse> {
        match serde_json::from_str::<Value>(response_body) {
            Ok(raw_response) => {
                let mut choices = Vec::new();
                let response_id = generate_id("gemini_resp");

                if let Some(candidates) = raw_response.get("candidates").and_then(|c| c.as_array()) {
                    if candidates.is_empty() {
                        return handle_empty_candidates(&raw_response, response_body);
                    }

                    for (index, candidate) in candidates.iter().enumerate() {
                        if index > 0 {
                            warn!("Handling only the first candidate from Gemini response.");
                            break;
                        }

                        let finish_reason = candidate
                            .get("finishReason")
                            .and_then(|fr| fr.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        if !["STOP", "MAX_TOKENS", "TOOL_CALLS"].contains(&finish_reason.as_str()) {
                            return handle_non_standard_finish_reason(
                                &raw_response,
                                &finish_reason,
                                response_body,
                            );
                        }

                        if let Some(content) = candidate.get("content") {
                            if let Some(role) = content.get("role").and_then(|r| r.as_str()) {
                                if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                                    let mut combined_text: Option<String> = None;
                                    let mut tool_calls: Option<Vec<ToolCall>> = None;
                                    let mut current_text = String::new();
                                    let mut current_tool_calls = Vec::new();

                                    for part in parts {
                                        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                            current_text.push_str(text);
                                        } else if let Some(function_call) = part.get("functionCall") {
                                            if let Some(name) = function_call.get("name").and_then(|n| n.as_str()) {
                                                if let Some(args) = function_call.get("args") {
                                                    current_tool_calls.push(ToolCall {
                                                        id: generate_id("gemini_tool"),
                                                        function: crate::models::tools::ToolFunction {
                                                            name: name.to_string(),
                                                            arguments: args.to_string(),
                                                        },
                                                        call_type: "function".to_string(),
                                                    });
                                                }
                                            }
                                        }
                                    }

                                    if !current_text.is_empty() {
                                        combined_text = Some(current_text);
                                    }
                                    if !current_tool_calls.is_empty() {
                                        tool_calls = Some(current_tool_calls);
                                    }

                                    let message_role = match role {
                                        "model" => "assistant".to_string(),
                                        _ => role.to_string(),
                                    };

                                    let message = ChatMessage {
                                        role: message_role,
                                        content: combined_text,
                                        tool_calls,
                                        tool_call_id: None,
                                    };

                                    choices.push(Choice {
                                        index: index as u32,
                                        message,
                                        finish_reason,
                                    });
                                }
                            }
                        }
                    }
                }

                if choices.is_empty() {
                    Err(anyhow!(
                        "Failed to extract choices from Gemini response structure: {}",
                        response_body
                    ))
                } else {
                    Ok(ApiResponse {
                        id: response_id,
                        choices,
                    })
                }
            }
            Err(e) => Err(anyhow!(e)).context(format!("Failed to parse Gemini response: {}", response_body)),
        }
    }

    fn build_headers(&self, api_key: &str) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "application/json".to_string(),
        );
        if !api_key.is_empty() {
            headers.insert(
                "x-goog-api-key".to_string(),
                api_key.to_string(),
            );
        }
        Ok(headers)
    }

    fn adapt_endpoint(&self, endpoint: &str, api_key: &str) -> Result<String> {
        let mut url = endpoint.to_string();
        if !api_key.is_empty() {
            url = format!("{}?key={}", url, api_key);
        }
        Ok(url)
    }
}

fn map_role_to_gemini(role: &str) -> Option<&str> {
    match role {
        "user" => Some("user"),
        "assistant" => Some("model"),
        "tool" => Some("function"),
        "system" => None,
        _ => {
            warn!(role = %role, "Unknown role encountered for Gemini mapping, skipping message.");
            None
        }
    }
}

fn generate_id(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}_{}", prefix, nanos)
}

fn handle_empty_candidates(raw_response: &Value, response_body: &str) -> Result<ApiResponse> {
    if let Some(feedback) = raw_response.get("promptFeedback") {
        if let Some(reason) = feedback.get("blockReason").and_then(|r| r.as_str()) {
            error!(block_reason = %reason, raw_response = %response_body, "Gemini request blocked (empty candidates).");
            return Err(anyhow!("Gemini request blocked due to: {} (empty candidates)", reason));
        }
        if let Some(ratings) = feedback.get("safetyRatings").and_then(|r| r.as_array()) {
            let high_severity_ratings: Vec<&Value> = ratings
                .iter()
                .filter(|rating| {
                    rating
                        .get("severity")
                        .and_then(|s| s.as_str())
                        .map_or(false, |s| s.starts_with("HIGH"))
                })
                .collect();
            if !high_severity_ratings.is_empty() {
                let reason_details = high_severity_ratings
                    .iter()
                    .map(|r| format!("{:?}", r))
                    .collect::<Vec<String>>()
                    .join(", ");
                error!(
                    safety_ratings = %reason_details,
                    raw_response = %response_body,
                    "Gemini request likely blocked due to high severity safety ratings (empty candidates)."
                );
                return Err(anyhow!(
                    "Gemini request blocked due to safety ratings: {} (empty candidates)",
                    reason_details
                ));
            }
        }
    }
    Err(anyhow!(
        "Failed to extract choices from Gemini response structure (candidates array was empty). Raw Response: {}",
        response_body
    ))
}

fn handle_non_standard_finish_reason(
    raw_response: &Value,
    finish_reason: &str,
    response_body: &str,
) -> Result<ApiResponse> {
    warn!(
        finish_reason = %finish_reason,
        raw_response = %response_body,
        "Gemini candidate finishReason indicates potential issue (e.g., safety block)."
    );
    if let Some(feedback) = raw_response.get("promptFeedback") {
        if let Some(reason) = feedback.get("blockReason").and_then(|r| r.as_str()) {
            error!(
                block_reason = %reason,
                finish_reason = %finish_reason,
                raw_response = %response_body,
                "Gemini request blocked (reported via finishReason/blockReason)."
            );
            return Err(anyhow!(
                "Gemini request blocked due to: {} (finishReason: {})",
                reason,
                finish_reason
            ));
        }
        if let Some(ratings) = feedback.get("safetyRatings").and_then(|r| r.as_array()) {
            let high_severity_ratings: Vec<&Value> = ratings
                .iter()
                .filter(|rating| {
                    rating
                        .get("severity")
                        .and_then(|s| s.as_str())
                        .map_or(false, |s| s.starts_with("HIGH"))
                })
                .collect();
            if !high_severity_ratings.is_empty() {
                let reason_details = high_severity_ratings
                    .iter()
                    .map(|r| format!("{:?}", r))
                    .collect::<Vec<String>>()
                    .join(", ");
                error!(
                    safety_ratings = %reason_details,
                    finish_reason = %finish_reason,
                    raw_response = %response_body,
                    "Gemini request likely blocked due to high severity safety ratings (reported via finishReason)."
                );
                return Err(anyhow!(
                    "Gemini request blocked due to safety ratings: {} (finishReason: {})",
                    reason_details,
                    finish_reason
                ));
            }
        }
    }
    Err(anyhow!(
        "Gemini response candidate indicates non-standard completion (finishReason: {}). Raw Response: {}",
        finish_reason,
        response_body
    ))
} 