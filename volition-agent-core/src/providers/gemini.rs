// volition-agent-core/src/providers/gemini.rs
use super::Provider; // Import Provider trait
use crate::api; // Use refactored api module
use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;

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
}

#[async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &str {
        &self.config.model_name
    }

    async fn get_completion(&self, messages: Vec<ChatMessage>) -> Result<ApiResponse> {
        let endpoint = self.config.endpoint.as_deref()
             .ok_or_else(|| anyhow!("Endpoint missing for Gemini provider model {}", self.config.model_name))?;
             
        // Call the generic API function
        // TODO: Handle tool conversion properly later
        api::call_chat_completion_api(
            &self.http_client,
            endpoint,
            &self.api_key,
            &self.config.model_name,
            messages,
            None, // Pass None for tools for now
            self.config.parameters.as_ref(),
        ).await
    }
}
