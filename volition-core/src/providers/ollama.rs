// volition-agent-core/src/providers/ollama.rs
use super::Provider;
use crate::api;
use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::models::tools::ToolDefinition; // Import ToolDefinition
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;

#[derive(Clone)]
pub struct OllamaProvider {
    config: ModelConfig,
    http_client: Client,
    // No API key needed for standard Ollama
}

impl OllamaProvider {
    // API key is ignored here
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

    // Add tools argument
    async fn get_completion(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>, // Add tools argument
    ) -> Result<ApiResponse> {
        let endpoint = self.config.endpoint.as_deref().ok_or_else(|| {
            anyhow!(
                "Endpoint missing for Ollama provider model {}",
                self.config.model_name
            )
        })?;

        // Call the generic API function
        // Ollama API might differ slightly (e.g., no bearer auth, different tool format?)
        // Assuming call_chat_completion_api is compatible or needs adjustment later.
        // Pass empty string for API key as it's not used.
        api::call_chat_completion_api(
            &self.http_client,
            endpoint,
            "", // No API key for Ollama
            &self.config.model_name,
            messages,
            tools,                           // Pass tools argument down
            self.config.parameters.as_ref(), // Restore parameters
        )
        .await
    }
}
