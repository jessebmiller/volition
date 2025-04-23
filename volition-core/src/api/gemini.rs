use super::ChatApiProvider;
use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::models::tools::ToolDefinition;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use toml::Value as TomlValue;

pub const DEFAULT_ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent";

pub struct GeminiProvider {
    api_key: String,
    endpoint: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, endpoint: Option<String>) -> Self {
        Self {
            api_key,
            endpoint: endpoint.unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
        }
    }
}

#[async_trait]
impl ChatApiProvider for GeminiProvider {
    fn build_payload(
        &self,
        model_name: &str,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
        parameters: Option<&TomlValue>,
    ) -> Result<Value> {
        // Convert messages to Gemini format
        let contents = messages
            .iter()
            .map(|msg| {
                json!({
                    "role": msg.role,
                    "parts": [{"text": msg.content}]
                })
            })
            .collect::<Vec<_>>();

        // Build the base payload
        let mut payload = json!({
            "model": model_name,
            "contents": contents,
            "generationConfig": {
                "temperature": 0.7,
                "topP": 0.8,
                "topK": 40,
            }
        });

        // Add tools if provided
        if let Some(tools) = tools {
            let tools_json = tools
                .iter()
                .map(|tool| {
                    json!({
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    })
                })
                .collect::<Vec<_>>();
            payload["tools"] = json!(tools_json);
        }

        // Add any additional parameters using more idiomatic Rust
        parameters
            .and_then(|p| p.get("generation_config"))
            .and_then(|v| v.as_table())
            .map(|table| {
                table.iter()
                    .filter_map(|(key, value)| value.as_float().map(|num| (key, num)))
                    .for_each(|(key, num)| {
                        payload["generationConfig"][key] = json!(num);
                    });
            });

        Ok(payload)
    }

    fn parse_response(&self, response_body: &str) -> Result<ApiResponse> {
        let response: Value = serde_json::from_str(response_body)?;
        
        // Extract the generated text
        let content = response["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("No text content in response"))?
            .to_string();

        // Extract the finish reason
        let finish_reason = response["candidates"][0]["finishReason"]
            .as_str()
            .unwrap_or("stop")
            .to_string();

        // Extract usage information if available
        let prompt_tokens = response["usageMetadata"]["promptTokenCount"].as_i64().unwrap_or(0) as u32;
        let completion_tokens = response["usageMetadata"]["candidatesTokenCount"].as_i64().unwrap_or(0) as u32;
        let total_tokens = prompt_tokens + completion_tokens;

        // Create a choice from the response
        let choice = Choice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: Some(content.clone()),
                tool_calls: None,
                tool_call_id: None,
            },
            finish_reason: finish_reason.clone(),
        };

        Ok(ApiResponse {
            id: response["model"].as_str().unwrap_or("gemini-pro").to_string(),
            content,
            finish_reason,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            choices: vec![choice],
        })
    }

    fn build_headers(&self) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        if !self.api_key.is_empty() {
            headers.insert("x-goog-api-key".to_string(), self.api_key.clone());
        }
        Ok(headers)
    }

    fn get_endpoint(&self) -> String {
        self.endpoint.clone()
    }
}
