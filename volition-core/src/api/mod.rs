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
    fn build_headers(&self) -> Result<HashMap<String, String>>;

    /// Gets the endpoint URL for the API request
    fn get_endpoint(&self) -> String;
}

pub mod gemini;
pub mod openai;
pub mod ollama;

/// Generic function to make a request to an AI chat completion API
pub async fn call_chat_completion_api(
    http_client: &Client,
    provider: Box<dyn ChatApiProvider>,
    model_name: &str,
    messages: Vec<ChatMessage>,
    tools: Option<&[ToolDefinition]>,
    parameters: Option<&TomlValue>,
) -> Result<ApiResponse> {
    println!("[DEBUG] Starting API call...");
    println!("[DEBUG] Model name: {}", model_name);
    
    // Use the provider to build the request
    let endpoint = provider.get_endpoint();
    println!("[DEBUG] Endpoint: {}", endpoint);
    
    let headers = provider.build_headers()?;
    println!("[DEBUG] Headers: {:?}", headers);
    
    let payload = provider.build_payload(model_name, messages, tools, parameters)?;
    println!("[DEBUG] Payload: {}", serde_json::to_string_pretty(&payload)?);

    // Make the HTTP request
    let mut header_map = HeaderMap::new();
    for (key, value) in headers {
        if let (Ok(name), Ok(val)) = (HeaderName::from_bytes(key.as_bytes()), HeaderValue::from_str(&value)) {
            header_map.insert(name, val);
        }
    }
    println!("[DEBUG] Sending HTTP request...");
    let response = http_client
        .post(&endpoint)
        .headers(header_map)
        .json(&payload)
        .send()
        .await?;

    // Parse the response
    let status = response.status();
    println!("[DEBUG] Response status: {}", status);
    let response_text = response.text().await?;
    println!("[DEBUG] Response body: {}", response_text);

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "API request failed with status {}: {}",
            status,
            response_text
        ));
    }

    println!("[DEBUG] Parsing response...");
    let result = provider.parse_response(&response_text);
    match &result {
        Ok(_) => println!("[DEBUG] Response parsed successfully"),
        Err(e) => println!("[DEBUG] Failed to parse response: {}", e),
    }
    result
}

#[cfg(test)]
mod tests; 