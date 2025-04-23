// volition-agent-core/src/providers/gemini.rs
use super::Provider;
use crate::api;
use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::models::tools::ToolDefinition;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

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
        if let Some(endpoint) = &self.config.endpoint {
            endpoint.clone()
        } else {
            format!("{}/{}:generateContent?key={}", DEFAULT_BASE_URL, self.config.model_name, self.api_key)
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
        tools: Option<&[ToolDefinition]>,
    ) -> Result<ApiResponse> {
        println!("[DEBUG] Building Gemini endpoint...");
        let endpoint = self.build_endpoint();
        println!("[DEBUG] Endpoint: {}", endpoint);
        println!("[DEBUG] Model name: {}", self.config.model_name);
        println!("[DEBUG] API key length: {}", self.api_key.len());

        let provider = Box::new(api::gemini::GeminiProvider::new(
            self.api_key.clone(),
            Some(endpoint),
            self.config.model_name.clone(),
        ));

        println!("[DEBUG] Calling chat completion API...");
        let result = api::call_chat_completion_api(
            &self.http_client,
            provider,
            &self.config.model_name,
            messages,
            tools,
            self.config.parameters.as_ref(),
        )
        .await;
        
        match &result {
            Ok(_) => println!("[DEBUG] API call successful"),
            Err(e) => println!("[DEBUG] API call failed: {}", e),
        }
        
        result
    }
}
