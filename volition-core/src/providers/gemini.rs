// volition-agent-core/src/providers/gemini.rs
use super::Provider;
use crate::api;
use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::models::tools::ToolDefinition;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use tracing::{error, trace}; // Removed info, warn

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

    async fn get_completion(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>, // Use tools argument again
    ) -> Result<ApiResponse> {
        trace!("Entering GeminiProvider::get_completion");
        let endpoint = self.config.endpoint.as_deref().ok_or_else(|| {
            anyhow!(
                "Endpoint missing for Gemini provider model {}",
                self.config.model_name
            )
        })?;
        trace!(endpoint = %endpoint, "Endpoint retrieved.");

        // Restore passing tools if available
        // warn!("TEMPORARY: Sending request to Gemini without tools.");

        trace!("Calling api::call_chat_completion_api...");
        let result = api::call_chat_completion_api(
            &self.http_client,
            endpoint,
            &self.api_key,
            &self.config.model_name,
            messages,
            tools,                           // Pass tools argument down
            self.config.parameters.as_ref(), // Restore parameters
        )
        .await;

        match &result {
            Ok(_) => trace!("api::call_chat_completion_api returned Ok"),
            Err(e) => error!(error = %e, "api::call_chat_completion_api returned Err"),
        }

        result // Return the original result
    }
}
