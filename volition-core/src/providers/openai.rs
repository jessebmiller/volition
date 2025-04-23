// volition-agent-core/src/providers/openai.rs
use super::Provider;
use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::models::tools::ToolDefinition;
use anyhow::{Result, anyhow, Context};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::{debug, warn};

const DEFAULT_OPENAI_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Clone)]
pub struct OpenAIProvider {
    config: ModelConfig,
    http_client: Client,
    api_key: String,
}

impl OpenAIProvider {
    pub fn new(config: ModelConfig, http_client: Client, api_key: String) -> Self {
        Self {
            config,
            http_client,
            api_key,
        }
    }

    fn build_payload(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
    ) -> Result<Value> {
        debug!("Building OpenAI payload...");
        debug!("Model name: {}", self.config.model_name);
        debug!("Message count: {}", messages.len());

        let mut payload = json!({
            "model": self.config.model_name,
            "messages": messages.iter().map(|msg| {
                json!({
                    "role": msg.role,
                    "content": msg.content.as_deref().unwrap_or_default()
                })
            }).collect::<Vec<_>>()
        });

        // Add tools if present
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let functions: Vec<Value> = tools
                    .iter()
                    .map(|t| {
                        json!({
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters
                        })
                    })
                    .collect();
                payload["functions"] = json!(functions);
                payload["function_call"] = json!("auto");
            }
        }

        // Add parameters if present
        if let Some(params) = &self.config.parameters {
            if let Some(temperature) = params.get("temperature").and_then(|t| t.as_float()) {
                payload["temperature"] = json!(temperature);
            }
            // Add other OpenAI-specific parameters here if needed
        }

        debug!("Final payload: {}", serde_json::to_string_pretty(&payload)?);
        Ok(payload)
    }

    fn parse_response(&self, response_body: &str) -> Result<ApiResponse> {
        debug!("Parsing OpenAI response...");
        debug!("Response body: {}", response_body);

        let raw_response: Value = serde_json::from_str(response_body)?;
        
        let choice = &raw_response["choices"][0];
        let message = &choice["message"];

        let content = message["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing content in OpenAI response"))?
            .to_string();
        debug!("Extracted content: {}", content);

        let finish_reason = choice["finish_reason"]
            .as_str()
            .unwrap_or("stop")
            .to_string();
        debug!("Finish reason: {}", finish_reason);

        let usage = &raw_response["usage"];
        let prompt_tokens = usage["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = usage["completion_tokens"].as_u64().unwrap_or(0) as u32;
        let total_tokens = usage["total_tokens"].as_u64().unwrap_or(0) as u32;
        debug!("Token usage - prompt: {}, completion: {}, total: {}", 
            prompt_tokens, completion_tokens, total_tokens);

        let mut tool_calls = None;
        if let Some(function_call) = message.get("function_call") {
            if let (Some(name), Some(arguments)) = (
                function_call["name"].as_str(),
                function_call["arguments"].as_str(),
            ) {
                tool_calls = Some(vec![crate::models::tools::ToolCall {
                    id: format!("call_{}", name),
                    call_type: "function".to_string(),
                    function: crate::models::tools::ToolFunction {
                        name: name.to_string(),
                        arguments: arguments.to_string(),
                    },
                }]);
            }
        }

        let result = ApiResponse {
            id: raw_response["id"]
                .as_str()
                .map(|s| s.to_string())
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
                    tool_calls,
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
        let endpoint = self.config.endpoint.as_deref().unwrap_or_else(|| {
            warn!("No endpoint specified for OpenAI provider model {}, using default: {}", self.config.model_name, DEFAULT_OPENAI_ENDPOINT);
            DEFAULT_OPENAI_ENDPOINT
        });

        if self.api_key.is_empty() {
            warn!(
                "API key is empty for OpenAI provider model {}. The API call will likely fail.",
                self.config.model_name
            );
        }

        let payload = self.build_payload(messages, tools)?;

        let response = self
            .http_client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        let response_body = response
            .text()
            .await
            .context("Failed to read response from OpenAI API")?;

        self.parse_response(&response_body)
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
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
