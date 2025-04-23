// volition-agent-core/src/providers/openai.rs
use super::Provider;
use crate::api;
use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::models::tools::ToolDefinition;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use tracing::warn;

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

        let provider = Box::new(api::openai::OpenAIProvider::new(
            self.api_key.clone(),
            Some(endpoint.to_string()),
        ));

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
