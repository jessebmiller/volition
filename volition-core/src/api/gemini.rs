use super::ChatApiProvider;
use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::models::tools::ToolDefinition;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use toml::Value as TomlValue;

pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub const DEFAULT_ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub struct GeminiProvider {
    api_key: String,
    endpoint: Option<String>,
    model_name: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, endpoint: Option<String>, model_name: String) -> Self {
        Self {
            api_key,
            endpoint,
            model_name,
        }
    }

    fn get_effective_endpoint(&self) -> String {
        self.endpoint.clone().unwrap_or_else(|| {
            format!("{}/{}:generateContent?key={}", DEFAULT_ENDPOINT, self.model_name, self.api_key)
        })
    }

    fn build_endpoint(&self) -> String {
        format!("https://generativelanguage.googleapis.com/v1/models/{}:generateContent", self.model_name)
    }
}

#[async_trait]
impl ChatApiProvider for GeminiProvider {
    fn build_payload(
        &self,
        _model_name: &str,
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
                    "parts": [{"text": msg.content}],
                })
            })
            .collect::<Vec<_>>();

        // Build the base payload
        let mut payload = json!({
            "contents": contents,
            "generationConfig": {
                "responseMimeType": "text/plain"
            },
        });

        // Add tools if present
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let tools_json: Vec<Value> = tools
                    .iter()
                    .map(|t| {
                        json!({
                            "functionDeclarations": [{
                                "name": t.name,
                                "description": t.description,
                                "parameters": t.parameters
                            }]
                        })
                    })
                    .collect();
                payload["tools"] = json!(tools_json);
            }
        }

        // Add any additional parameters
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

        let result = ApiResponse {
            id: response["model"].as_str().unwrap_or(&self.model_name).to_string(),
            content,
            finish_reason,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            choices: vec![choice],
        };
        
        Ok(result)
    }

    fn build_headers(&self) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(headers)
    }

    fn get_endpoint(&self) -> String {
        self.get_effective_endpoint()
    }
}
