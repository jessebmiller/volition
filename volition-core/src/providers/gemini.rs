// volition-agent-core/src/providers/gemini.rs
use super::Provider;
use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::models::tools::ToolDefinition;
use anyhow::{Result, anyhow, Context};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::debug;

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Clone)]
pub struct GeminiProvider {
    config: ModelConfig,
    http_client: Client,
    api_key: String,
}

impl GeminiProvider {
    pub fn new(config: ModelConfig, http_client: Client, api_key: String) -> Self {
        Self {
            config,
            http_client,
            api_key,
        }
    }

    fn build_endpoint(&self) -> String {
        debug!("Building Gemini endpoint...");
        if let Some(endpoint) = &self.config.endpoint {
            endpoint.clone()
        } else {
            format!("{}/{}:generateContent?key={}", DEFAULT_BASE_URL, self.config.model_name, self.api_key)
        }
    }

    fn build_payload(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
    ) -> Result<Value> {
        debug!("Building Gemini payload...");
        debug!("Model name: {}", self.config.model_name);
        debug!("Message count: {}", messages.len());

        let mut payload = json!({
            "contents": messages.iter().map(|msg| {
                json!({
                    "role": msg.role,
                    "parts": [{"text": msg.content.as_deref().unwrap_or_default()}]
                })
            }).collect::<Vec<_>>()
        });

        // Add tools if present
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let tools_with_type: Vec<Value> = tools
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
                payload["tools"] = json!(tools_with_type);
            }
        }

        // Add parameters if present
        if let Some(params) = &self.config.parameters {
            if let Some(generation_config) = params.get("generation_config") {
                if let Some(table) = generation_config.as_table() {
                    debug!("Adding generation config parameters");
                    let mut generation_config = json!({});
                    for (key, value) in table {
                        if let Some(num) = value.as_float() {
                            generation_config[key] = json!(num);
                        }
                    }
                    payload["generationConfig"] = generation_config;
                }
            }
        }

        debug!("Final payload: {}", serde_json::to_string_pretty(&payload)?);
        Ok(payload)
    }

    fn parse_response(&self, response_body: &str) -> Result<ApiResponse> {
        debug!("Parsing Gemini response...");
        debug!("Response body: {}", response_body);

        let raw_response: Value = serde_json::from_str(response_body)?;
        
        let content = raw_response["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing text content in Gemini response"))?
            .to_string();
        debug!("Extracted content: {}", content);

        let finish_reason = raw_response["candidates"][0]["finishReason"]
            .as_str()
            .unwrap_or("stop")
            .to_string();
        debug!("Finish reason: {}", finish_reason);

        let prompt_tokens = raw_response["usageMetadata"]["promptTokenCount"]
            .as_u64()
            .unwrap_or(0) as u32;
        let completion_tokens = raw_response["usageMetadata"]["candidatesTokenCount"]
            .as_u64()
            .unwrap_or(0) as u32;
        let total_tokens = prompt_tokens + completion_tokens;
        debug!("Token usage - prompt: {}, completion: {}, total: {}", 
            prompt_tokens, completion_tokens, total_tokens);

        let result = ApiResponse {
            id: raw_response["promptFeedback"]["promptTokenCount"]
                .as_u64()
                .map(|n| n.to_string())
                .unwrap_or_default(),
            content: content.clone(),
            finish_reason: finish_reason.clone(),
            prompt_tokens,
            completion_tokens,
            total_tokens,
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: Some(content),
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason,
            }],
        };
        
        debug!("Parsed response: {:?}", result);
        Ok(result)
    }

    async fn call_chat_completion_api(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
    ) -> Result<ApiResponse> {
        let endpoint = self.build_endpoint();
        let payload = self.build_payload(messages, tools)?;

        let response = self
            .http_client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        let response_body = response
            .text()
            .await
            .context("Failed to read response from Gemini API")?;

        self.parse_response(&response_body)
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &str {
        &self.config.model_name
    }

    async fn get_completion(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
    ) -> Result<ApiResponse> {
        self.call_chat_completion_api(messages, tools).await
    }
}
