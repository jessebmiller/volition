// volition-agent-core/src/providers/ollama.rs
use super::Provider;
use crate::api;
use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::models::tools::ToolDefinition;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;

#[derive(Clone)]
pub struct OllamaProvider {
    config: ModelConfig,
    http_client: Client,
}

impl OllamaProvider {
    pub fn new(config: ModelConfig, http_client: Client, _api_key: String) -> Self {
        Self {
            config,
            http_client,
        }
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
        let endpoint = self.config.endpoint.as_deref().ok_or_else(|| {
            anyhow!(
                "Endpoint missing for Ollama provider model {}",
                self.config.model_name
            )
        })?;

        let provider = Box::new(api::ollama::OllamaProvider::new(endpoint.to_string()));

        api::call_chat_completion_api(
            &self.http_client,
            provider,
            &self.config.model_name,
            messages,
            tools,
            self.config.parameters.as_ref(),
        )
        .await
    }
}
