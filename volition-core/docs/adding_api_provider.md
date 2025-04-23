# How to Add an API Provider

This guide explains how to add a new API provider to the Volition core system. We'll use Anthropic as an example, but the same pattern applies to other API providers.

## 1. Create the Provider File

Create a new file in `volition-core/src/providers/` named after your provider (e.g., `anthropic.rs`).

## 2. Define the Provider Structure

```rust
use super::Provider;
use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::models::tools::{ToolCall, ToolDefinition};
use anyhow::{Result, anyhow, Context};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::debug;

pub struct AnthropicProvider {
    config: ModelConfig,
    http_client: Client,
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(config: ModelConfig, http_client: Client, api_key: String) -> Self {
        debug!("Creating new Anthropic provider with model: {}", config.model_name);
        Self {
            config,
            http_client,
            api_key,
        }
    }
}
```

## 3. Implement the Provider Trait

Implement the `Provider` trait for your provider:

```rust
#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        &self.config.model_name
    }

    async fn get_completion(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
    ) -> Result<ApiResponse> {
        self.call_chat_completion_api(messages, tools).await
    }
}
```

## 4. Implement API Methods

Add the necessary methods for API communication:

```rust
impl AnthropicProvider {
    fn build_payload(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
    ) -> Result<Value> {
        debug!("Building Anthropic payload...");
        debug!("Model name: {}", self.config.model_name);
        debug!("Message count: {}", messages.len());

        let mut payload = json!({
            "model": self.config.model_name,
            "messages": messages.iter().map(|msg| {
                json!({
                    "role": msg.role,
                    "content": msg.content.as_deref().unwrap_or_default()
                })
            }).collect::<Vec<_>>()
        });

        // Add tools if present
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let tools_with_type: Vec<Value> = tools
                    .iter()
                    .map(|t| {
                        json!({
                            "type": "function",
                            "function": {
                                "name": t.name,
                                "description": t.description,
                                "parameters": t.parameters
                            }
                        })
                    })
                    .collect();
                payload["tools"] = json!(tools_with_type);
            }
        }

        // Add parameters if present
        if let Some(params) = &self.config.parameters {
            if let Some(temperature) = params.get("temperature").and_then(|t| t.as_float()) {
                payload["temperature"] = json!(temperature);
            }
        }

        debug!("Final payload: {}", serde_json::to_string_pretty(&payload)?);
        Ok(payload)
    }

    fn parse_response(&self, response_body: &str) -> Result<ApiResponse> {
        debug!("Parsing Anthropic response...");
        debug!("Response body: {}", response_body);

        let raw_response: Value = serde_json::from_str(response_body)?;
        
        // Parse the response into ApiResponse format
        // This will be specific to the Anthropic API response structure
        todo!("Implement response parsing")
    }

    async fn call_chat_completion_api(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
    ) -> Result<ApiResponse> {
        let endpoint = self.config.endpoint.as_deref().unwrap_or("https://api.anthropic.com/v1/messages");
        debug!("Using Anthropic endpoint: {}", endpoint);

        let payload = self.build_payload(messages, tools)?;

        debug!("Sending request to Anthropic API...");
        let response = self
            .http_client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;

        debug!("Received response from Anthropic API, status: {}", response.status());
        let response_body = response
            .text()
            .await
            .context("Failed to read response from Anthropic API")?;

        self.parse_response(&response_body)
    }
}
```

## Key Considerations

1. **Error Handling**: Implement proper error handling for API requests and responses.
2. **Rate Limiting**: Consider implementing rate limiting if the API has restrictions.
3. **Authentication**: Handle API authentication appropriately using the provider's `api_key` field.
4. **Response Parsing**: Ensure proper parsing of API responses into the `ApiResponse` format.
5. **Tool Support**: If the API supports tools/functions, implement the necessary mapping.
6. **Testing**: Add tests for your provider implementation.
7. **Configuration**: Support both default and custom endpoints through the provider's constructor.

## Example Implementation

For a complete example, see the implementation of other providers in the codebase:

- `volition-core/src/providers/openai.rs`
- `volition-core/src/providers/gemini.rs`
- `volition-core/src/providers/ollama.rs`

## Testing Your Provider

1. Create unit tests for your provider implementation
2. Test error cases and edge conditions
3. Test tool/function calling if supported
4. Test rate limiting and retry logic if implemented
5. Test both default and custom endpoint configurations

## Adding to the Build System

If your provider requires additional dependencies, add them to `volition-core/Cargo.toml`:

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
# Add other dependencies as needed
``` 