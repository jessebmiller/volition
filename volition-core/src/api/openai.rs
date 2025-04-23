use super::ChatApiProvider;
use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::models::tools::{ToolCall, ToolDefinition};
use anyhow::{Result, anyhow, Context};
use serde_json::{json, Value};
use std::collections::HashMap;
use toml::Value as TomlValue;
use tracing::warn;

pub struct OpenAIProvider;

impl OpenAIProvider {
    pub fn new() -> Self {
        Self
    }
}

impl ChatApiProvider for OpenAIProvider {
    fn build_payload(
        &self,
        model_name: &str,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
        parameters: Option<&TomlValue>,
    ) -> Result<Value> {
        let mut payload = json!({
            "model": model_name,
            "messages": messages
        });

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
                payload["tools"] = json!(tools_with_type);
            }
        }

        // Add parameters if present
        if let Some(params) = parameters {
            if let Some(temperature) = params.get("temperature").and_then(|t| t.as_float()) {
                payload["temperature"] = json!(temperature);
            }
        }

        Ok(payload)
    }

    fn parse_response(&self, response_body: &str) -> Result<ApiResponse> {
        match serde_json::from_str::<Value>(response_body) {
            Ok(raw_response) => {
                let response_id = raw_response
                    .get("id")
                    .and_then(|id| id.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        warn!("No ID in OpenAI response, generating one");
                        generate_id("openai_resp")
                    });

                let choices = if let Some(choices_array) = raw_response.get("choices").and_then(|c| c.as_array()) {
                    choices_array
                        .iter()
                        .enumerate()
                        .filter_map(|(index, choice)| {
                            let message = choice.get("message")?;
                            let role = message.get("role")?.as_str()?;
                            let content = message.get("content").and_then(|c| c.as_str());
                            let tool_calls = message.get("tool_calls").and_then(|tc| tc.as_array()).map(|tc| {
                                tc.iter().filter_map(|tc| {
                                    let id = tc.get("id")?.as_str()?;
                                    let function = tc.get("function")?;
                                    let name = function.get("name")?.as_str()?;
                                    let arguments = function.get("arguments")?.as_str()?;
                                    Some(ToolCall {
                                        id: id.to_string(),
                                        function: crate::models::tools::ToolFunction {
                                            name: name.to_string(),
                                            arguments: arguments.to_string(),
                                        },
                                        call_type: "function".to_string(),
                                    })
                                }).collect()
                            });
                            let finish_reason = choice.get("finish_reason")?.as_str()?;

                            Some(Choice {
                                index: index as u32,
                                message: ChatMessage {
                                    role: role.to_string(),
                                    content: content.map(|s| s.to_string()),
                                    tool_calls,
                                    tool_call_id: None,
                                },
                                finish_reason: finish_reason.to_string(),
                            })
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                if choices.is_empty() {
                    Err(anyhow!(
                        "Failed to extract choices from OpenAI response structure: {}",
                        response_body
                    ))
                } else {
                    Ok(ApiResponse {
                        id: response_id,
                        choices,
                    })
                }
            }
            Err(e) => Err(anyhow!(e)).context(format!("Failed to parse OpenAI response: {}", response_body)),
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
                "Authorization".to_string(),
                format!("Bearer {}", api_key),
            );
        }
        Ok(headers)
    }

    fn adapt_endpoint(&self, endpoint: &str, _api_key: &str) -> Result<String> {
        Ok(endpoint.to_string())
    }
}

fn generate_id(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}_{}", prefix, nanos)
} 