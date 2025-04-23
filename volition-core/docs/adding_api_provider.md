# How to Add an API Provider

This guide explains how to add a new API provider to the Volition core system. We'll use Anthropic as an example, but the same pattern applies to other API providers.

## 1. Create the Provider File

Create a new file in `volition-core/src/api/` named after your provider (e.g., `anthropic.rs`).

## 2. Define the Provider Structure

```rust
use super::ChatApiProvider;
use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::models::tools::{ToolCall, ToolDefinition};
use anyhow::{Result, anyhow, Context};
use serde_json::{json, Value};
use std::collections::HashMap;
use toml::Value as TomlValue;
use tracing::warn;

pub struct AnthropicProvider;

impl AnthropicProvider {
    pub fn new() -> Self {
        Self
    }
}
```

## 3. Implement the ChatApiProvider Trait

Implement the `ChatApiProvider` trait for your provider:

```rust
impl ChatApiProvider for AnthropicProvider {
    fn build_payload(
        &self,
        model_name: &str,
        messages: Vec<ChatMessage>,
        tools: Option<&[ToolDefinition]>,
        parameters: Option<&TomlValue>,
    ) -> Result<Value> {
        let mut payload = json!({
            "model": model_name,
            "messages": messages
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
        if let Some(params) = parameters {
            if let Some(temperature) = params.get("temperature").and_then(|t| t.as_float()) {
                payload["temperature"] = json!(temperature);
            }
        }

        Ok(payload)
    }

    fn parse_response(&self, response_body: &str) -> Result<ApiResponse> {
        match serde_json::from_str::<Value>(response_body) {
            Ok(raw_response) => {
                // Parse the response into ApiResponse format
                // This will be specific to the Anthropic API response structure
                todo!("Implement response parsing")
            }
            Err(e) => Err(anyhow!(e)).context(format!(
                "Failed to parse Anthropic response: {}", 
                response_body
            )),
        }
    }

    fn build_headers(&self, api_key: &str) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "application/json".to_string(),
        );
        if !api_key.is_empty() {
            headers.insert(
                "Authorization".to_string(),
                format!("Bearer {}", api_key),
            );
        }
        Ok(headers)
    }

    fn adapt_endpoint(&self, endpoint: &str, _api_key: &str) -> Result<String> {
        Ok(endpoint.to_string())
    }
}
```

## 4. Expose the Module

Add your provider module to `volition-core/src/api/mod.rs`:

```rust
pub mod anthropic;
```

## 5. Update Provider Selection

Update the provider selection in `call_chat_completion_api` in `volition-core/src/api/mod.rs`:

```rust
let provider: Box<dyn ChatApiProvider> = if endpoint_str.contains("googleapis.com") {
    Box::new(gemini::GeminiProvider::new())
} else if endpoint_str.contains("openai.com") {
    Box::new(openai::OpenAIProvider::new())
} else if endpoint_str.contains("anthropic.com") {
    Box::new(anthropic::AnthropicProvider::new())
} else {
    Box::new(ollama::OllamaProvider::new())
};
```

## Key Considerations

1. **Error Handling**: Implement proper error handling for API requests and responses.
2. **Rate Limiting**: Consider implementing rate limiting if the API has restrictions.
3. **Authentication**: Handle API authentication appropriately.
4. **Response Parsing**: Ensure proper parsing of API responses into the `ApiResponse` format.
5. **Tool Support**: If the API supports tools/functions, implement the necessary mapping.
6. **Testing**: Add tests for your provider implementation.

## Example Implementation

For a complete example, see the implementation of other providers in the codebase:

- `volition-core/src/api/openai.rs`
- `volition-core/src/api/gemini.rs`
- `volition-core/src/api/ollama.rs`

## Testing Your Provider

1. Create unit tests for your provider implementation
2. Test error cases and edge conditions
3. Test tool/function calling if supported
4. Test rate limiting and retry logic if implemented

## Adding to the Build System

If your provider requires additional dependencies, add them to `volition-core/Cargo.toml`:

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
# Add other dependencies as needed
``` 