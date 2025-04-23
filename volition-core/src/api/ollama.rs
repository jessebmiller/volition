use super::ChatApiProvider;
use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::models::tools::ToolDefinition;
use anyhow::{Result, anyhow, Context};
use serde_json::{json, Value};
use std::collections::HashMap;
use toml::Value as TomlValue;
use async_trait::async_trait;

pub struct OllamaProvider {
    endpoint: String,
}

impl OllamaProvider {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }
}

#[async_trait]
impl ChatApiProvider for OllamaProvider {
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

        // Add tools if present (Ollama might not support tools yet, but we'll include them for future compatibility)
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
                let response_id = generate_id("ollama_resp");

                let choices = if let Some(message) = raw_response.get("message") {
                    let role = message.get("role").and_then(|r| r.as_str()).unwrap_or("assistant");
                    let content = message.get("content").and_then(|c| c.as_str());

                    vec![Choice {
                        index: 0,
                        message: ChatMessage {
                            role: role.to_string(),
                            content: content.map(|s| s.to_string()),
                            tool_calls: None,
                            tool_call_id: None,
                        },
                        finish_reason: "stop".to_string(),
                    }]
                } else {
                    Vec::new()
                };

                if choices.is_empty() {
                    Err(anyhow!(
                        "Failed to extract message from Ollama response structure: {}",
                        response_body
                    ))
                } else {
                    Ok(ApiResponse {
                        id: response_id,
                        content: choices[0].message.content.clone().unwrap_or_default(),
                        finish_reason: choices[0].finish_reason.clone(),
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                        choices,
                    })
                }
            }
            Err(e) => Err(anyhow!(e)).context(format!("Failed to parse Ollama response: {}", response_body)),
        }
    }

    fn build_headers(&self) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "application/json".to_string(),
        );
        Ok(headers)
    }

    fn get_endpoint(&self) -> String {
        self.endpoint.clone()
    }
}

fn generate_id(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}_{}", prefix, nanos)
} 