// volition-agent-core/src/providers/ollama.rs
use super::Provider;
use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::models::tools::ToolDefinition;
use anyhow::{Result, anyhow, Context};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::debug;

#[derive(Clone)]
pub struct OllamaProvider {
    config: ModelConfig,
    http_client: Client,
}

impl OllamaProvider {
    pub fn new(config: ModelConfig, http_client: Client, _api_key: String) -> Self {
        debug!("Creating new Ollama provider with model: {}", config.model_name);
        Self {
            config,
            http_client,
        }
    }

    fn build_payload(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
    ) -> Result<Value> {
        debug!("Building Ollama payload...");
        debug!("Model name: {}", self.config.model_name);
        debug!("Message count: {}", messages.len());
        debug!("Tools present: {}", tools.is_some());

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
                debug!("Adding tools to payload for model {}", self.config.model_name);
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
        if let Some(params) = &self.config.parameters {
            debug!("Adding parameters to payload");
            if let Some(temperature) = params.get("temperature").and_then(|t| t.as_float()) {
                payload["temperature"] = json!(temperature);
                debug!("Added temperature: {}", temperature);
            }
            // Add other Ollama-specific parameters here if needed
        }

        // Always set stream to false to disable streaming
        payload["stream"] = json!(false);

        debug!("Final payload: {}", serde_json::to_string_pretty(&payload)?);
        Ok(payload)
    }

    fn parse_response(&self, response_body: &str) -> Result<ApiResponse> {
        debug!("Parsing Ollama response...");
        debug!("Response body: {}", response_body);

        let raw_response: Value = serde_json::from_str(response_body)?;
        
        let content = raw_response["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing content in Ollama response"))?
            .to_string();
        debug!("Extracted content: {}", content);

        let result = ApiResponse {
            id: raw_response["model"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_default(),
            content: content.clone(),
            finish_reason: "stop".to_string(), // Ollama doesn't provide this
            prompt_tokens: 0, // Ollama doesn't provide token counts
            completion_tokens: 0,
            total_tokens: 0,
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: Some(content),
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: "stop".to_string(),
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
        let endpoint = self.config.endpoint.as_deref().unwrap_or("http://127.0.0.1:11434/api/chat");
        debug!("Using Ollama endpoint: {}", endpoint);

        let payload = self.build_payload(messages, tools)?;

        debug!("Sending request to Ollama API...");
        let response = self
            .http_client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to Ollama API")?;

        debug!("Received response from Ollama API, status: {}", response.status());
        let response_body = response
            .text()
            .await
            .context("Failed to read response from Ollama API")?;

        self.parse_response(&response_body)
    }
}

#[async_trait]
impl Provider for OllamaProvider {
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
