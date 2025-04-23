use super::ChatApiProvider;
use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::models::tools::ToolDefinition;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use toml::Value as TomlValue;

pub struct OpenAIProvider {
    api_key: String,
    endpoint: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, endpoint: Option<String>) -> Self {
        Self {
            api_key,
            endpoint: endpoint.unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string()),
        }
    }
}

#[async_trait]
impl ChatApiProvider for OpenAIProvider {
    fn build_payload(
        &self,
        model_name: &str,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
        parameters: Option<&TomlValue>,
    ) -> Result<Value> {
        // Build the base payload
        let mut payload = json!({
            "model": model_name,
            "messages": messages,
            "temperature": 0.7,
            "top_p": 0.8,
        });

        // Add tools if provided
        if let Some(tools) = tools {
            let tools_json = tools
                .iter()
                .map(|tool| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.parameters
                        }
                    })
                })
                .collect::<Vec<_>>();
            payload["tools"] = json!(tools_json);
        }

        // Add any additional parameters
        if let Some(params) = parameters {
            if let Some(toml::Value::Table(table)) = params.get("generation_config") {
                for (key, value) in table {
                    if let Some(num) = value.as_float() {
                        payload[key] = json!(num);
                    }
                }
            }
        }

        Ok(payload)
    }

    fn parse_response(&self, response_body: &str) -> Result<ApiResponse> {
        let response: Value = serde_json::from_str(response_body)?;
        
        // Extract the generated text
        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("No text content in response"))?
            .to_string();

        // Extract the finish reason
        let finish_reason = response["choices"][0]["finish_reason"]
            .as_str()
            .unwrap_or("stop")
            .to_string();

        // Extract usage information
        let prompt_tokens = response["usage"]["prompt_tokens"].as_i64().unwrap_or(0) as u32;
        let completion_tokens = response["usage"]["completion_tokens"].as_i64().unwrap_or(0) as u32;
        let total_tokens = response["usage"]["total_tokens"].as_i64().unwrap_or(0) as u32;

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
            id: response["id"].as_str().unwrap_or("").to_string(),
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
            headers.insert("Authorization".to_string(), format!("Bearer {}", self.api_key));
        }
        Ok(headers)
    }

    fn get_endpoint(&self) -> String {
        self.endpoint.clone()
    }
} 