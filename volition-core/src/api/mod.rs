use crate::models::chat::{ApiResponse, ChatMessage};
use crate::models::tools::ToolDefinition;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, header::{HeaderMap, HeaderName, HeaderValue}};
use serde_json::Value;
use std::collections::HashMap;
use toml::Value as TomlValue;

#[async_trait]
pub trait ChatApiProvider: Send + Sync {
    /// Builds the request payload for the specific API provider
    fn build_payload(
        &self,
        model_name: &str,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
        parameters: Option<&TomlValue>,
    ) -> Result<Value>;

    /// Parses the API response into our common ApiResponse format
    fn parse_response(&self, response_body: &str) -> Result<ApiResponse>;

    /// Builds the headers for the API request
    fn build_headers(&self, api_key: &str) -> Result<HashMap<String, String>>;

    /// Adapts the endpoint URL if needed (e.g., adding query parameters)
    fn adapt_endpoint(&self, endpoint: &str, api_key: &str) -> Result<String>;
}

pub mod gemini;
pub mod openai;
pub mod ollama;

/// Generic function to make a request to an AI chat completion API
pub async fn call_chat_completion_api(
    http_client: &Client,
    endpoint_str: &str,
    api_key: &str,
    model_name: &str,
    messages: Vec<ChatMessage>,
    tools: Option<&[ToolDefinition]>,
    parameters: Option<&TomlValue>,
) -> Result<ApiResponse> {
    // Determine which provider to use based on the endpoint
    let provider: Box<dyn ChatApiProvider> = if endpoint_str.contains("googleapis.com") {
        Box::new(gemini::GeminiProvider::new())
    } else if endpoint_str.contains("openai.com") {
        Box::new(openai::OpenAIProvider::new())
    } else {
        Box::new(ollama::OllamaProvider::new())
    };

    // Use the provider to build the request
    let endpoint = provider.adapt_endpoint(endpoint_str, api_key)?;
    let headers = provider.build_headers(api_key)?;
    let payload = provider.build_payload(model_name, messages, tools, parameters)?;

    // Make the HTTP request
    let mut header_map = HeaderMap::new();
    for (key, value) in headers {
        if let (Ok(name), Ok(val)) = (HeaderName::from_bytes(key.as_bytes()), HeaderValue::from_str(&value)) {
            header_map.insert(name, val);
        }
    }
    let response = http_client
        .post(&endpoint)
        .headers(header_map)
        .json(&payload)
        .send()
        .await?;

    // Parse the response
    let status = response.status();
    let response_text = response.text().await?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "API request failed with status {}: {}",
            status,
            response_text
        ));
    }

    provider.parse_response(&response_text)
} 