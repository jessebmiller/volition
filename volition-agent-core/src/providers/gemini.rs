// volition-agent-core/src/providers/gemini.rs
use super::{Provider, ProviderRegistry}; // Import necessary items
use crate::api; // Use existing api module
use crate::config::ModelConfig; // Use existing config types
use crate::models::chat::{ApiResponse, ChatMessage};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

#[derive(Clone)] // Clone needed if storing directly in Agent
pub struct GeminiProvider {
    config: ModelConfig,
    http_client: Client,
    api_key: String, // Store API key
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
        // Use the specific model name from config
        &self.config.model_name 
    }

    async fn get_completion(&self, messages: Vec<ChatMessage>) -> Result<ApiResponse> {
        // Use the existing generic API call function, passing necessary details
        // We might need to adapt get_chat_completion or create a provider-specific one
        // For now, assume get_chat_completion can handle it with the right config
        // NOTE: get_chat_completion currently doesn't take api_key directly
        // It reads from RuntimeConfig. This needs refactoring.
        
        // Placeholder - requires config/api refactoring
         Err(anyhow::anyhow!(
            "GeminiProvider::get_completion needs config/api refactoring to pass API key/endpoint correctly"
        ))
        
        /* // Example of intended usage after refactoring:
        api::call_provider_api(
            &self.http_client,
            &self.config.endpoint, 
            &self.api_key, 
            messages,
            &self.config.model_name, 
            self.config.parameters.get("temperature").and_then(|v| v.as_f64()),
            // Add other parameters as needed
        ).await
        */
    }
}
