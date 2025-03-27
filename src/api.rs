use anyhow::{anyhow, Context, Result}; // Added Context
use reqwest::Client;
use serde_json::{error::Category as SerdeJsonCategory, json, to_value, Value};
// Removed HashMap import as overrides are gone
use tokio::time::Duration;
use tracing::{debug, warn};
// Removed unused url::Url import
use uuid::Uuid;

use crate::models::chat::{ApiResponse, ResponseMessage};
use crate::models::tools::Tools;
// Use the combined RuntimeConfig and ModelConfig
use crate::config::{ModelConfig, RuntimeConfig};

/// Unified function to send chat requests to an OpenAI-compatible endpoint.
/// Constructs the URL, request body, and headers based on the provided ModelConfig.
pub async fn chat_with_endpoint(
    client: &Client,
    config: &RuntimeConfig,     // Pass the full config for API key access
    model_config: &ModelConfig, // Use the specific model config
    messages: Vec<ResponseMessage>,
) -> Result<ApiResponse> {
    // Use the endpoint directly from ModelConfig as it now contains the full path.
    // The URL validation is done during config loading.
    let url_str = &model_config.endpoint;

    // Build the request body using the OpenAI format.
    let request_body = build_openai_request(&model_config.model_name, messages, model_config)?;

    debug!(
        "Request URL: {}\nRequest JSON: {}",
        url_str,
        serde_json::to_string_pretty(&request_body)?
    );

    // Exponential backoff parameters (remain unchanged)
    let max_retries = 5;
    let initial_delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(60);
    let backoff_factor = 2.0;

    let mut retries = 0;
    let mut delay = initial_delay;

    loop {
        // Always add Content-Type and Authorization headers.
        let request = client
            .post(url_str) // Use the endpoint string directly
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", config.api_key)); // Use API key from RuntimeConfig

        let response_result = request.json(&request_body).send().await;

        let response = match response_result {
            Ok(resp) => resp,
            Err(e) => {
                // Retry on network errors (unchanged)
                if retries < max_retries {
                    retries += 1;
                    warn!(
                        "Network error sending request: {}. Retrying in {} seconds (attempt {}/{})",
                        e,
                        delay.as_secs(),
                        retries,
                        max_retries
                    );
                    // Use a very small sleep in tests if possible, or configure via env var?
                    // For now, rely on test client timeout.
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(
                        Duration::from_secs((delay.as_secs() as f64 * backoff_factor) as u64),
                        max_delay,
                    );
                    continue;
                } else {
                    return Err(anyhow!(
                        "Network error after {} retries: {}",
                        max_retries,
                        e
                    ));
                }
            }
        };

        let status = response.status();

        // Retry on rate limiting and server errors (unchanged)
        // Use as_u16() for status code comparison
        if (status.as_u16() == 429 || status.is_server_error()) && retries < max_retries {
            let retry_after = if let Some(retry_header) = response.headers().get("retry-after") {
                if let Ok(retry_secs) = retry_header.to_str().unwrap_or("0").parse::<u64>() {
                    Some(Duration::from_secs(retry_secs))
                } else {
                    None
                }
            } else {
                None
            };

            let wait_time = retry_after.unwrap_or(delay);
            retries += 1;
            warn!(
                "API request failed with status {}. Retrying in {} seconds (attempt {}/{})",
                status,
                wait_time.as_secs(),
                retries,
                max_retries
            );
             // Use a very small sleep in tests if possible, or configure via env var?
             // For now, rely on test client timeout.
            tokio::time::sleep(wait_time).await;
            delay = std::cmp::min(
                Duration::from_secs((delay.as_secs() as f64 * backoff_factor) as u64),
                max_delay,
            );
            continue;
        }

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .context("Failed to read API error response body")?;
            return Err(anyhow!("API error: {} - {}", status, error_text));
        }

        // Deserialize response, handling potential missing 'choices'
        let response_value: Value = response
            .json()
            .await
            .context("Failed to read API response body as JSON")?; // Added context

        // Inject 'id' if missing
        let mut response_json_obj = if let Value::Object(map) = response_value {
            map
        } else {
            // If the top level isn't an object, we can't deserialize into ApiResponse anyway.
            return Err(anyhow!(
                "API response was not a JSON object: {:?}",
                response_value // Use the original value here for the error
            ));
        };

        if !response_json_obj.contains_key("id") {
            let new_id = format!("chatcmpl-{}", Uuid::new_v4());
            debug!(
                "Added missing 'id' field to API response with value: {}",
                new_id
            );
            response_json_obj.insert("id".to_string(), json!(new_id));
        }

        // Now attempt deserialization from the potentially modified JSON object
        let api_response_result: Result<ApiResponse, serde_json::Error> =
            serde_json::from_value(Value::Object(response_json_obj.clone())); // Clone needed if we log below

        let api_response = match api_response_result {
            Ok(resp) => resp,
            Err(e) => {
                // Check if the error is data-related (like missing fields) and matches our specific case
                if e.classify() == SerdeJsonCategory::Data
                    && e.to_string().contains("missing field `choices`")
                {
                    // Log the problematic JSON for debugging
                    warn!(
                        "API response successfully received but missing 'choices' field. Response body: {}",
                        serde_json::to_string_pretty(&response_json_obj).unwrap_or_else(|_| format!("{:?}", response_json_obj))
                    );
                    // Return a specific error
                    return Err(anyhow!(
                        "API call succeeded but response was missing the expected 'choices' field."
                    )
                    .context(e)); // Add original serde error as context
                } else {
                    // For any other deserialization error, wrap and return
                    return Err(anyhow!("Failed to deserialize API response").context(e));
                }
            }
        };

        // Debug logging for response (unchanged)
        debug!("=== API RESPONSE ===");
        if let Some(choices) = api_response.choices.first() {
            if let Some(tool_calls) = &choices.message.tool_calls {
                debug!("Tool calls: {:#?}", tool_calls);
            } else {
                debug!("No tool calls");
            }
        } else {
            // This case should be less likely now unless choices is empty,
            // as a missing choices field is handled above.
            debug!("Response has empty 'choices' array or first choice has no message/tool_calls");
        }
        debug!("=====================");

        return Ok(api_response);
    }
}

/// Builds the request body for OpenAI-compatible chat completion endpoints.
fn build_openai_request(
    model_name: &str,
    messages: Vec<ResponseMessage>,
    model_config: &ModelConfig, // Keep ModelConfig for parameters access
) -> Result<Value> {
    let mut request_map = serde_json::Map::new();
    request_map.insert("model".to_string(), json!(model_name));
    request_map.insert("messages".to_string(), to_value(messages)?);

    // Always add tools, as we standardized on the OpenAI interface which supports them.
    request_map.insert(
        "tools".to_string(),
        json!([
            Tools::shell_definition(),
            Tools::read_file_definition(),
            Tools::write_file_definition(),
            Tools::search_text_definition(),
            Tools::find_rust_definition_definition(), // Updated function call
            Tools::user_input_definition(),
            Tools::git_command_definition(),
            Tools::cargo_command_definition(),
            Tools::list_directory_definition()
        ]),
    );

    // Add parameters from model_config.parameters (unchanged)
    if let Some(parameters) = model_config.parameters.as_table() {
        for (key, value) in parameters {
            // Using to_value ensures TOML values are correctly converted to JSON values
            let json_value = to_value(value.clone())
                .with_context(|| format!("Failed to convert TOML parameter '{}' to JSON", key))?;
            request_map.insert(key.clone(), json_value);
        }
    }

    Ok(Value::Object(request_map))
}

/// Selects the model based on RuntimeConfig and delegates the API call to chat_with_endpoint.
pub async fn chat_with_api(
    client: &Client,
    config: &RuntimeConfig, // Use the combined RuntimeConfig
    messages: Vec<ResponseMessage>,
    // Removed overrides parameter
) -> Result<ApiResponse> {
    // No more effective_config or override logic needed.

    // Retrieve the selected model key directly from the top-level config.
    let selected_model_key = &config.selected_model;

    // Retrieve the configuration for the selected model.
    // The config loader already validated that this key exists.
    let model_config = config.models.get(selected_model_key).ok_or_else(|| {
        anyhow!(
            // Should not happen if config loading is correct, but good practice
            "Internal error: Selected model key '{}' not found in models map after config load.",
            selected_model_key
        )
    })?;

    // No more service matching or validation needed here.

    // Call the unified endpoint function, passing the full config and the specific model_config.
    chat_with_endpoint(client, config, model_config, messages).await
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ModelConfig, RuntimeConfig}; // Ensure these are in scope
    use crate::models::chat::ResponseMessage;
    // Note: ToolCall might not be needed directly if ResponseMessage construction is simple
    // use crate::models::tools::ToolCall;
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use toml; // Import toml for creating parameters
    use std::time::Duration; // Import Duration for client timeout
    // use std::sync::atomic::{AtomicUsize, Ordering}; // Removed stateful mock imports
    // use std::sync::Arc;

    // Add imports for httpmock and tokio test
    use httpmock::prelude::*;
    use tokio; // Make sure tokio is in scope for the test attribute


    // Helper to create a basic ModelConfig for testing build_openai_request
    fn create_test_model_config(endpoint: &str, params: Option<toml::value::Table>) -> ModelConfig {
        ModelConfig {
            model_name: "test-model-name".to_string(), // Use a consistent test model name
            endpoint: endpoint.to_string(),
            parameters: params.map(toml::Value::Table).unwrap_or(toml::Value::Table(toml::value::Table::new())),
        }
    }

     // Helper to create a basic RuntimeConfig for tests needing it
    fn create_test_runtime_config(selected_key: &str, model_config: ModelConfig) -> RuntimeConfig {
        let mut models = HashMap::new();
        models.insert(selected_key.to_string(), model_config);
        RuntimeConfig {
            system_prompt: "Test prompt".to_string(),
            selected_model: selected_key.to_string(),
            models,
            api_key: "default-test-api-key".to_string(), // Default key
            project_root: PathBuf::from("/fake/path"),
        }
    }


    #[test]
    fn test_build_openai_request_basic() {
        let model_name = "gpt-basic";
        let messages = vec![ResponseMessage {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];
        let model_config = create_test_model_config("http://fake.endpoint/v1", None);

        let result = build_openai_request(model_name, messages.clone(), &model_config);
        assert!(result.is_ok());
        let value = result.unwrap();

        assert_eq!(value["model"], json!(model_name));
        assert_eq!(value["messages"], json!(messages));
        assert!(value["tools"].is_array());
        assert!(value["tools"].as_array().unwrap().len() > 5); // Check that tools are included
        assert!(value.get("temperature").is_none()); // No parameters added
    }

    #[test]
    fn test_build_openai_request_with_parameters() {
        let model_name = "gpt-params";
        let messages = vec![ResponseMessage {
            role: "user".to_string(),
            content: Some("Test".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];
        let mut params = toml::value::Table::new();
        params.insert("temperature".to_string(), toml::Value::Float(0.9));
        params.insert("max_tokens".to_string(), toml::Value::Integer(100));
        let model_config = create_test_model_config("http://fake.endpoint/v1", Some(params));


        let result = build_openai_request(model_name, messages.clone(), &model_config);
        assert!(result.is_ok());
        let value = result.unwrap();

        assert_eq!(value["model"], json!(model_name));
        assert_eq!(value["messages"], json!(messages));
        assert!(value["tools"].is_array());
        assert_eq!(value["temperature"], json!(0.9));
        assert_eq!(value["max_tokens"], json!(100));
    }

     #[test]
    fn test_build_openai_request_includes_all_tools() {
        let model_name = "gpt-tools-check";
        let messages = vec![ResponseMessage { role: "user".to_string(), content: Some("Test".to_string()), tool_calls: None, tool_call_id: None }];
        let model_config = create_test_model_config("http://fake.endpoint/v1", None);

        let result = build_openai_request(model_name, messages, &model_config);
        assert!(result.is_ok());
        let value = result.unwrap();
        let tools_array = value["tools"].as_array().expect("Tools should be an array");

        // Check for the presence of *some* expected tool names/function names
        let tool_names: Vec<String> = tools_array.iter()
            .filter_map(|t| t.get("function").and_then(|f| f.get("name")))
            .filter_map(|n| n.as_str().map(String::from))
            .collect();

        assert!(tool_names.contains(&"shell".to_string()));
        assert!(tool_names.contains(&"read_file".to_string()));
        assert!(tool_names.contains(&"write_file".to_string()));
        assert!(tool_names.contains(&"search_text".to_string()));
        assert!(tool_names.contains(&"find_rust_definition".to_string()));
        assert!(tool_names.contains(&"user_input".to_string()));
        assert!(tool_names.contains(&"git_command".to_string()));
        assert!(tool_names.contains(&"cargo_command".to_string()));
        assert!(tool_names.contains(&"list_directory".to_string()));
        assert_eq!(tool_names.len(), 9, "Expected 9 tools to be defined"); // Ensure no extra/missing tools
    }

    // --- Tests for chat_with_endpoint ---

    #[tokio::test]
    async fn test_chat_with_endpoint_success() {
        // Arrange
        let server = MockServer::start_async().await;
        let api_key = "test-success-key";
        let model_key = "default_test_model";
        let endpoint_path = "/v1/chat/completions";
        let server_url = server.base_url();
        let full_endpoint_url = format!("{}{}", server_url, endpoint_path);
        let messages = vec![ResponseMessage { role: "user".to_string(), content: Some("Ping".to_string()), tool_calls: None, tool_call_id: None }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let runtime_config = RuntimeConfig { api_key: api_key.to_string(), ..runtime_config };
        let specific_model_config = runtime_config.models.get(model_key).unwrap();

        let mock = server.mock_async(|when, then| {
            when.method(POST)
                .path(endpoint_path)
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {}", api_key))
                .json_body(build_openai_request(&specific_model_config.model_name, messages.clone(), specific_model_config).unwrap());
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({
                    "id": "chatcmpl-123",
                    "object": "chat.completion",
                    "created": 1677652288,
                    "model": specific_model_config.model_name,
                    "choices": [{
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": "Pong",
                        },
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 9,
                        "completion_tokens": 12,
                        "total_tokens": 21
                    }
                }));
        }).await;

        let client = Client::new();
        let result = chat_with_endpoint(&client, &runtime_config, specific_model_config, messages).await;

        // Assert
        mock.assert_async().await; // Asserts hits == 1 by default
        assert!(result.is_ok(), "Expected Ok result, got Err: {:?}", result.err());
        let response = result.unwrap();
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.content, Some("Pong".to_string()));
        assert_eq!(response.choices[0].message.role, "assistant");
    }

    #[tokio::test]
    async fn test_chat_with_endpoint_401_unauthorized() {
        // Arrange
        let server = MockServer::start_async().await;
        let api_key = "invalid-api-key";
        let model_key = "default_test_model";
        let endpoint_path = "/v1/chat/completions";
        let server_url = server.base_url();
        let full_endpoint_url = format!("{}{}", server_url, endpoint_path);
        let messages = vec![ResponseMessage { role: "user".to_string(), content: Some("Test".to_string()), tool_calls: None, tool_call_id: None }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let runtime_config = RuntimeConfig { api_key: api_key.to_string(), ..runtime_config };
        let specific_model_config = runtime_config.models.get(model_key).unwrap();

        // Mock the 401 response
        let mock = server.mock_async(|when, then| {
            when.method(POST)
                .path(endpoint_path)
                .header("Authorization", &format!("Bearer {}", api_key));
            then.status(401)
                .header("Content-Type", "application/json")
                .body("{\"error\": \"Invalid API key\"}");
        }).await;

        let client = Client::new();
        let result = chat_with_endpoint(&client, &runtime_config, specific_model_config, messages).await;

        // Assert
        assert_eq!(mock.hits(), 1); // Check hits AFTER action
        assert!(result.is_err(), "Expected Err result, but got Ok");
        let error = result.err().unwrap();
        let error_string = error.to_string();
        assert!(error_string.contains("API error: 401 Unauthorized"), "Error message mismatch: {}", error_string);
        assert!(error_string.contains("Invalid API key"), "Error message mismatch: {}", error_string);
    }

    #[tokio::test]
    async fn test_chat_with_endpoint_500_retry_and_fail() {
        // Arrange
        let server = MockServer::start_async().await;
        let api_key = "test-retry-key";
        let model_key = "default_test_model";
        let endpoint_path = "/v1/chat/completions";
        let server_url = server.base_url();
        let full_endpoint_url = format!("{}{}", server_url, endpoint_path);
        let messages = vec![ResponseMessage { role: "user".to_string(), content: Some("Test Retry".to_string()), tool_calls: None, tool_call_id: None }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let runtime_config = RuntimeConfig { api_key: api_key.to_string(), ..runtime_config };
        let specific_model_config = runtime_config.models.get(model_key).unwrap();

        // Mock the 500 response
        let mock = server.mock_async(|when, then| {
            when.method(POST)
                .path(endpoint_path);
            then.status(500)
                .body("Server error");
        }).await;

        let client = Client::builder()
             .timeout(Duration::from_millis(100)) // Short timeout for test
             .build().unwrap();

        // Act
        let result = chat_with_endpoint(&client, &runtime_config, specific_model_config, messages).await;

        // Assert
        assert_eq!(mock.hits(), 6); // Check hits AFTER action
        assert!(result.is_err(), "Expected Err result after retries, but got Ok");
        let error = result.err().unwrap();
        let error_string = error.to_string();
        assert!(error_string.contains("API error: 500 Internal Server Error"), "Error message mismatch: {}", error_string);
        assert!(error_string.contains("Server error"), "Error message mismatch: {}", error_string);
    }

    #[tokio::test]
    async fn test_chat_with_endpoint_missing_choices() {
        // Arrange
        let server = MockServer::start_async().await;
        let api_key = "test-missing-choices-key";
        let model_key = "default_test_model";
        let endpoint_path = "/v1/chat/completions";
        let server_url = server.base_url();
        let full_endpoint_url = format!("{}{}", server_url, endpoint_path);

        let messages = vec![ResponseMessage { role: "user".to_string(), content: Some("Test missing choices".to_string()), tool_calls: None, tool_call_id: None }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let runtime_config = RuntimeConfig { api_key: api_key.to_string(), ..runtime_config };
        let specific_model_config = runtime_config.models.get(model_key).unwrap();

        // Mock a 200 response but without the 'choices' field
        let mock = server.mock_async(|when, then| {
            when.method(POST)
                .path(endpoint_path);
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(json!({
                    "id": "chatcmpl-789", // Need an ID, otherwise the code adds one
                    "object": "chat.completion",
                    "created": 1677652299,
                    "model": specific_model_config.model_name,
                    // "choices": [], // Deliberately missing
                    "usage": {
                        "prompt_tokens": 10,
                        "completion_tokens": 0,
                        "total_tokens": 10
                    }
                }));
        }).await;

        let client = Client::new();

        // Act
        let result = chat_with_endpoint(&client, &runtime_config, specific_model_config, messages).await;

        // Assert
        mock.assert_async().await; // Should be hit once
        assert!(result.is_err(), "Expected Err result due to missing choices, but got Ok");

        // Check the specific error message
        let error = result.err().unwrap();
        let error_string = format!("{:?}", error); // Use debug format to see context potentially
        assert!(
            error_string.contains("API call succeeded but response was missing the expected 'choices' field"),
            "Error message mismatch: {}", error_string
        );
        // Also check that the underlying serde error is mentioned in the context
        assert!(
            error_string.contains("missing field `choices`"),
            "Underlying serde error context missing: {}", error_string
        );
    }


    // --- Test for chat_with_api ---

    #[tokio::test]
    async fn test_chat_with_api_selects_correct_model() {
        // Arrange
        let server = MockServer::start_async().await;
        let api_key = "test-selector-key";
        let endpoint_path_a = "/v1/model_a";
        let endpoint_path_b = "/v1/model_b"; // The one we expect to be called
        let server_url = server.base_url();

        // Config for Model A (not selected)
        let model_config_a = ModelConfig {
            model_name: "model-a-name".to_string(),
            endpoint: format!("{}{}", server_url, endpoint_path_a),
            parameters: toml::Value::Table(toml::value::Table::new()),
        };

        // Config for Model B (selected)
        let model_config_b = ModelConfig {
            model_name: "model-b-name".to_string(), // Different name
            endpoint: format!("{}{}", server_url, endpoint_path_b), // Different endpoint
            parameters: toml::Value::Table(toml::value::Table::new()),
        };

        // Create RuntimeConfig with both models, but selecting 'model_b'
        let mut models = HashMap::new();
        models.insert("model_a".to_string(), model_config_a);
        models.insert("model_b".to_string(), model_config_b.clone()); // Clone config_b for use in mock setup

        let runtime_config = RuntimeConfig {
            system_prompt: "Selector test".to_string(),
            selected_model: "model_b".to_string(), // <--- Select model_b
            models,
            api_key: api_key.to_string(),
            project_root: PathBuf::from("/fake/selector"),
        };

        let messages = vec![ResponseMessage { role: "user".to_string(), content: Some("Select test".to_string()), tool_calls: None, tool_call_id: None }];

        // Mock: Expect a request ONLY to model_b's endpoint and with model_b's details
        let mock_b = server.mock_async(|when, then| {
            when.method(POST)
                .path(endpoint_path_b) // Expect call to model_b's path
                .header("Authorization", &format!("Bearer {}", api_key))
                // Check body uses model_b's name
                .json_body(build_openai_request(&model_config_b.model_name, messages.clone(), &model_config_b).unwrap());

            // Response doesn't strictly matter for this test, just that the right endpoint was called
            then.status(200)
                .json_body(json!({
                    "id": "chatcmpl-selected-b",
                    "choices": [{"index": 0, "message": {"role": "assistant", "content": "Selected B"}, "finish_reason": "stop"}]
                }));
        }).await;
        // We don't define a mock for endpoint_path_a. If it gets called, the test will fail.

        let client = Client::new();

        // Act: Call chat_with_api (which should delegate to chat_with_endpoint using model_b's config)
        let result = chat_with_api(&client, &runtime_config, messages).await;

        // Assert
        mock_b.assert_async().await; // Verify model_b's endpoint was hit exactly once
        assert!(result.is_ok(), "chat_with_api failed: {:?}", result.err());
        // Optional: Check response content matches the mock for completeness
        let response = result.unwrap();
        assert_eq!(response.id, "chatcmpl-selected-b");
        assert_eq!(response.choices[0].message.content, Some("Selected B".to_string()));
    }
}
