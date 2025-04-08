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
        // Use configured endpoint or default to OpenAI's standard endpoint
        let endpoint = self.config.endpoint.as_deref().unwrap_or_else(|| {
            warn!("No endpoint specified for OpenAI provider model {}, using default: {}", self.config.model_name, DEFAULT_OPENAI_ENDPOINT);
            DEFAULT_OPENAI_ENDPOINT
        });

        if self.api_key.is_empty() {
            // Although api::call_chat_completion_api warns, we add a specific check here
            // because OpenAI *always* requires a key.
            warn!(
                "API key is empty for OpenAI provider model {}. The API call will likely fail.",
                self.config.model_name
            );
            // Potentially return an error here instead of just warning?
            // For now, align with existing behaviour and let the API call fail.
            // return Err(anyhow!("API key is missing for OpenAI provider model {}", self.config.model_name));
        }

        // Call the generic API function
        api::call_chat_completion_api(
            &self.http_client,
            endpoint,
            &self.api_key, // Pass the API key
            &self.config.model_name,
            messages,
            tools,
            self.config.parameters.as_ref(),
        )
        .await
    }
}
