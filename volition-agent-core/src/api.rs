// volition-agent-core/src/api.rs

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::{error::Category as SerdeJsonCategory, json, to_value, Value};
use tokio::time::Duration;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::config::{ModelConfig, RuntimeConfig};
use crate::models::chat::{ApiResponse, ResponseMessage};
use crate::models::tools::ToolDefinition; // Import ToolDefinition
use crate::ToolProvider; // Import ToolProvider trait

/// Unified function to send chat requests to an OpenAI-compatible endpoint.
/// Constructs the URL, request body, and headers based on the provided ModelConfig.
/// Includes configurable retry logic.
pub async fn chat_with_endpoint(
    client: &Client,
    config: &RuntimeConfig,
    model_config: &ModelConfig,
    messages: Vec<ResponseMessage>,
    tool_provider: &dyn ToolProvider, // Added tool_provider
    max_retries: u32,
    initial_delay: Duration,
    max_delay: Duration,
) -> Result<ApiResponse> {
    let url_str = &model_config.endpoint;
    // Pass tool definitions from provider to build_openai_request
    let tool_definitions = tool_provider.get_tool_definitions();
    let request_body = build_openai_request(
        &model_config.model_name,
        messages,
        model_config,
        &tool_definitions, // Pass tool definitions
    )?;

    debug!(
        "Request URL: {}\nRequest JSON: {}",
        url_str,
        serde_json::to_string_pretty(&request_body)?
    );

    let backoff_factor = 2.0;
    let mut retries = 0;
    let mut current_delay = initial_delay;

    loop {
        let request = client
            .post(url_str)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", config.api_key));

        let response_result = request.json(&request_body).send().await;

        let response = match response_result {
            Ok(resp) => resp,
            Err(e) => {
                if retries < max_retries {
                    retries += 1;
                    warn!(
                        "Network error sending request: {}. Retrying in {:?} (attempt {}/{})",
                        e,
                        current_delay,
                        retries,
                        max_retries
                    );
                    tokio::time::sleep(current_delay).await;
                    current_delay = std::cmp::min(
                        Duration::from_secs_f64(current_delay.as_secs_f64() * backoff_factor),
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

            let wait_time = retry_after.unwrap_or(current_delay);
            retries += 1;
            warn!(
                "API request failed with status {}. Retrying in {:?} (attempt {}/{})",
                status, wait_time, retries, max_retries
            );
            tokio::time::sleep(wait_time).await;
            current_delay = std::cmp::min(
                Duration::from_secs_f64(current_delay.as_secs_f64() * backoff_factor),
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

        let response_value: Value = response
            .json()
            .await
            .context("Failed to read API response body as JSON")?;

        let mut response_json_obj = if let Value::Object(map) = response_value {
            map
        } else {
            return Err(anyhow!(
                "API response was not a JSON object: {:?}",
                response_value
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

        let api_response_result: Result<ApiResponse, serde_json::Error> =
            serde_json::from_value(Value::Object(response_json_obj.clone()));

        let api_response = match api_response_result {
            Ok(resp) => resp,
            Err(e) => {
                if e.classify() == SerdeJsonCategory::Data
                    && e.to_string().contains("missing field `choices`")
                {
                    warn!(
                        "API response successfully received but missing 'choices' field. Response body: {}",
                        serde_json::to_string_pretty(&response_json_obj).unwrap_or_else(|_| format!("{:?}", response_json_obj))
                    );
                    return Err(anyhow!(
                        "API call succeeded but response was missing the expected 'choices' field."
                    )
                    .context(e));
                } else {
                    return Err(anyhow!("Failed to deserialize API response").context(e));
                }
            }
        };

        debug!("=== API RESPONSE ===");
        if let Some(choices) = api_response.choices.first() {
            if let Some(tool_calls) = &choices.message.tool_calls {
                debug!("Tool calls: {:#?}", tool_calls);
            } else {
                debug!("No tool calls");
            }
        } else {
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
    model_config: &ModelConfig,
    tool_definitions: &[ToolDefinition], // Added tool_definitions parameter
) -> Result<Value> {
    let mut request_map = serde_json::Map::new();
    request_map.insert("model".to_string(), json!(model_name));
    request_map.insert("messages".to_string(), to_value(messages)?);

    // Convert ToolDefinition structs to the JSON format expected by the API
    let tools_json: Vec<Value> = tool_definitions
        .iter()
        .map(|tool_def| {
            json!({
                "type": "function",
                "function": tool_def // ToolDefinition struct already matches the required structure
            })
        })
        .collect();

    request_map.insert("tools".to_string(), Value::Array(tools_json));

    if let Some(parameters) = model_config.parameters.as_table() {
        for (key, value) in parameters {
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
    config: &RuntimeConfig,
    messages: Vec<ResponseMessage>,
    tool_provider: &dyn ToolProvider, // Added tool_provider
) -> Result<ApiResponse> {
    let selected_model_key = &config.selected_model;
    let model_config = config.models.get(selected_model_key).ok_or_else(|| {
        anyhow!(
            "Internal error: Selected model key '{}' not found in models map after config load.",
            selected_model_key
        )
    })?;

    // Define production retry parameters
    const PROD_MAX_RETRIES: u32 = 5;
    const PROD_INITIAL_DELAY: Duration = Duration::from_secs(1);
    const PROD_MAX_DELAY: Duration = Duration::from_secs(60);

    // Call the unified endpoint function, passing the tool provider and retry parameters.
    chat_with_endpoint(
        client,
        config,
        model_config,
        messages,
        tool_provider, // Pass tool provider along
        PROD_MAX_RETRIES,
        PROD_INITIAL_DELAY,
        PROD_MAX_DELAY,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ModelConfig, RuntimeConfig};
    use crate::models::chat::ResponseMessage;
    use crate::models::tools::{ToolDefinition, ToolInput, ToolParametersDefinition, ToolParameter, ToolParameterType};
    use crate::async_trait;
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use toml;
    use anyhow::Result;

    use httpmock::prelude::*;
    use tokio;

    // --- Mock Tool Provider for Tests ---
    struct MockToolProvider {
        definitions: Vec<ToolDefinition>,
    }

    #[async_trait]
    impl ToolProvider for MockToolProvider {
        fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
            self.definitions.clone()
        }

        async fn execute_tool(&self, _tool_name: &str, _input: ToolInput, _working_dir: &Path) -> Result<String> {
            // Not needed for api.rs tests
            unimplemented!("execute_tool should not be called in api tests")
        }
    }

    fn create_mock_tool_provider() -> MockToolProvider {
        let mut properties = HashMap::new();
        properties.insert("arg1".to_string(), ToolParameter {
             param_type: ToolParameterType::String,
             description: "Arg 1".to_string(),
             enum_values: None,
             items: None,
        });
        let mock_tool_def = ToolDefinition {
            name: "mock_tool".to_string(),
            description: "A mock tool".to_string(),
            parameters: ToolParametersDefinition {
                param_type: "object".to_string(),
                properties,
                required: vec!["arg1".to_string()],
            },
        };
        MockToolProvider { definitions: vec![mock_tool_def] }
    }
    // --- End Mock Tool Provider ---

    fn create_test_model_config(endpoint: &str, params: Option<toml::value::Table>) -> ModelConfig {
        ModelConfig {
            model_name: "test-model-name".to_string(),
            endpoint: endpoint.to_string(),
            parameters: params
                .map(toml::Value::Table)
                .unwrap_or(toml::Value::Table(toml::value::Table::new())),
        }
    }

    fn create_test_runtime_config(selected_key: &str, model_config: ModelConfig) -> RuntimeConfig {
        let mut models = HashMap::new();
        models.insert(selected_key.to_string(), model_config);
        RuntimeConfig {
            system_prompt: "Test prompt".to_string(),
            selected_model: selected_key.to_string(),
            models,
            api_key: "default-test-api-key".to_string(),
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
        let mock_provider = create_mock_tool_provider();
        let tool_definitions = mock_provider.get_tool_definitions();

        let result = build_openai_request(model_name, messages.clone(), &model_config, &tool_definitions);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["model"], json!(model_name));
        assert_eq!(value["messages"], json!(messages));
        assert!(value["tools"].is_array());
        // Check if the mock tool is included
        let tools_array = value["tools"].as_array().unwrap();
        assert_eq!(tools_array.len(), 1);
        assert_eq!(tools_array[0]["function"]["name"], "mock_tool");
        assert!(value.get("temperature").is_none());
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
        let mock_provider = create_mock_tool_provider();
        let tool_definitions = mock_provider.get_tool_definitions();

        let result = build_openai_request(model_name, messages.clone(), &model_config, &tool_definitions);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["model"], json!(model_name));
        assert_eq!(value["messages"], json!(messages));
        assert!(value["tools"].is_array());
        assert_eq!(value["temperature"], json!(0.9));
        assert_eq!(value["max_tokens"], json!(100));
    }

    // Remove the test that checked for specific hardcoded tools
    // #[test]
    // fn test_build_openai_request_includes_all_tools() { ... }

    // Define test retry parameters with very short delays
    const TEST_MAX_RETRIES: u32 = 5;
    const TEST_INITIAL_DELAY: Duration = Duration::from_millis(10);
    const TEST_MAX_DELAY: Duration = Duration::from_millis(50); // Keep max low too

    #[tokio::test]
    async fn test_chat_with_endpoint_success() {
        let server = MockServer::start_async().await;
        let api_key = "test-success-key";
        let model_key = "default_test_model";
        let endpoint_path = "/v1/chat/completions";
        let server_url = server.base_url();
        let full_endpoint_url = format!("{}{}", server_url, endpoint_path);
        let messages = vec![ResponseMessage {
            role: "user".to_string(),
            content: Some("Ping".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let runtime_config = RuntimeConfig {
            api_key: api_key.to_string(),
            ..runtime_config
        };
        let specific_model_config = runtime_config.models.get(model_key).unwrap();
        let mock_provider = create_mock_tool_provider();
        let tool_definitions = mock_provider.get_tool_definitions();

        let mock = server.mock_async(|when, then| {
            when.method(POST).path(endpoint_path).header("Authorization", &format!("Bearer {}", api_key))
                // Check request body includes the mock tool definitions
                .json_body(build_openai_request(&specific_model_config.model_name, messages.clone(), specific_model_config, &tool_definitions).unwrap());
            then.status(200).header("Content-Type", "application/json").json_body(json!({
                "id": "chatcmpl-123", "object": "chat.completion", "created": 1677652288, "model": specific_model_config.model_name,
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "Pong"}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 9, "completion_tokens": 12, "total_tokens": 21}
            }));
        }).await;

        let client = Client::new();
        let result = chat_with_endpoint(
            &client,
            &runtime_config,
            specific_model_config,
            messages,
            &mock_provider, // Pass mock provider
            TEST_MAX_RETRIES,
            TEST_INITIAL_DELAY,
            TEST_MAX_DELAY,
        )
        .await;

        mock.assert_async().await;
        assert!(
            result.is_ok(),
            "Expected Ok result, got Err: {:?}",
            result.err()
        );
        let response = result.unwrap();
        assert_eq!(response.id, "chatcmpl-123");
        // ... rest of assertions ...
    }

    #[tokio::test]
    async fn test_chat_with_endpoint_401_unauthorized() {
        let server = MockServer::start_async().await;
        let api_key = "invalid-api-key";
        let model_key = "default_test_model";
        let endpoint_path = "/v1/chat/completions";
        let server_url = server.base_url();
        let full_endpoint_url = format!("{}{}", server_url, endpoint_path);
        let messages = vec![ResponseMessage {
            role: "user".to_string(),
            content: Some("Test".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let runtime_config = RuntimeConfig {
            api_key: api_key.to_string(),
            ..runtime_config
        };
        let specific_model_config = runtime_config.models.get(model_key).unwrap();
        let mock_provider = create_mock_tool_provider(); // Needed for call signature

        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path(endpoint_path)
                    .header("Authorization", &format!("Bearer {}", api_key));
                then.status(401)
                    .header("Content-Type", "application/json")
                    .body("{\"error\": \"Invalid API key\"}");
            })
            .await;

        let client = Client::new();
        let result = chat_with_endpoint(
            &client,
            &runtime_config,
            specific_model_config,
            messages,
            &mock_provider, // Pass mock provider
            TEST_MAX_RETRIES,
            TEST_INITIAL_DELAY,
            TEST_MAX_DELAY,
        )
        .await;

        assert_eq!(mock.hits(), 1);
        assert!(result.is_err(), "Expected Err result, but got Ok");
        // ... rest of assertions ...
    }

    #[tokio::test]
    async fn test_chat_with_endpoint_500_retry_and_fail() {
        let server = MockServer::start_async().await;
        let api_key = "test-retry-key";
        let model_key = "default_test_model";
        let endpoint_path = "/v1/chat/completions";
        let server_url = server.base_url();
        let full_endpoint_url = format!("{}{}", server_url, endpoint_path);
        let messages = vec![ResponseMessage {
            role: "user".to_string(),
            content: Some("Test Retry".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let runtime_config = RuntimeConfig {
            api_key: api_key.to_string(),
            ..runtime_config
        };
        let specific_model_config = runtime_config.models.get(model_key).unwrap();
        let mock_provider = create_mock_tool_provider(); // Needed for call signature

        let mock = server
            .mock_async(|when, then| {
                when.method(POST).path(endpoint_path);
                then.status(500).body("Server error");
            })
            .await;

        let client = Client::builder().build().unwrap();

        let result = chat_with_endpoint(
            &client,
            &runtime_config,
            specific_model_config,
            messages,
            &mock_provider, // Pass mock provider
            TEST_MAX_RETRIES,
            TEST_INITIAL_DELAY,
            TEST_MAX_DELAY,
        )
        .await;

        assert_eq!(mock.hits(), TEST_MAX_RETRIES as usize + 1);
        assert!(
            result.is_err(),
            "Expected Err result after retries, but got Ok"
        );
        // ... rest of assertions ...
    }

    #[tokio::test]
    async fn test_chat_with_endpoint_missing_choices() {
        let server = MockServer::start_async().await;
        let api_key = "test-missing-choices-key";
        let model_key = "default_test_model";
        let endpoint_path = "/v1/chat/completions";
        let server_url = server.base_url();
        let full_endpoint_url = format!("{}{}", server_url, endpoint_path);
        let messages = vec![ResponseMessage {
            role: "user".to_string(),
            content: Some("Test missing choices".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let runtime_config = RuntimeConfig {
            api_key: api_key.to_string(),
            ..runtime_config
        };
        let specific_model_config = runtime_config.models.get(model_key).unwrap();
        let mock_provider = create_mock_tool_provider(); // Needed for call signature

        let mock = server.mock_async(|when, then| {
            when.method(POST).path(endpoint_path);
            then.status(200).header("Content-Type", "application/json").json_body(json!({
                "id": "chatcmpl-789", "object": "chat.completion", "created": 1677652299, "model": specific_model_config.model_name,
                "usage": {"prompt_tokens": 10, "completion_tokens": 0, "total_tokens": 10}
            }));
        }).await;

        let client = Client::new();
        let result = chat_with_endpoint(
            &client,
            &runtime_config,
            specific_model_config,
            messages,
            &mock_provider, // Pass mock provider
            TEST_MAX_RETRIES,
            TEST_INITIAL_DELAY,
            TEST_MAX_DELAY,
        )
        .await;

        mock.assert_async().await;
        assert!(
            result.is_err(),
            "Expected Err result due to missing choices, but got Ok"
        );
        // ... rest of assertions ...
    }

    #[tokio::test]
    async fn test_chat_with_api_selects_correct_model() {
        let server = MockServer::start_async().await;
        let api_key = "test-selector-key";
        let endpoint_path_b = "/v1/model_b";
        let server_url = server.base_url();

        let model_config_a = ModelConfig { /* ... */ model_name: "a".into(), endpoint: "a".into(), parameters: Default::default() }; // Dummy
        let model_config_b = create_test_model_config(&format!("{}{}", server_url, endpoint_path_b), None);

        let mut models = HashMap::new();
        models.insert("model_a".to_string(), model_config_a);
        models.insert("model_b".to_string(), model_config_b.clone());

        let runtime_config = RuntimeConfig {
            system_prompt: "Selector test".to_string(),
            selected_model: "model_b".to_string(),
            models,
            api_key: api_key.to_string(),
            project_root: PathBuf::from("/fake/selector"),
        };
        let messages = vec![ResponseMessage {
            role: "user".to_string(),
            content: Some("Select test".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];
        let mock_provider = create_mock_tool_provider(); // Needed for call signature
        let tool_definitions = mock_provider.get_tool_definitions();

        let mock_b = server.mock_async(|when, then| {
            when.method(POST).path(endpoint_path_b).header("Authorization", &format!("Bearer {}", api_key))
                .json_body(build_openai_request(&model_config_b.model_name, messages.clone(), &model_config_b, &tool_definitions).unwrap());
            then.status(200).json_body(json!({
                "id": "chatcmpl-selected-b", "choices": [{"index": 0, "message": {"role": "assistant", "content": "Selected B"}, "finish_reason": "stop"}]
            }));
        }).await;

        let client = Client::new();
        let result = chat_with_api(&client, &runtime_config, messages, &mock_provider).await; // Pass mock provider

        mock_b.assert_async().await;
        assert!(result.is_ok(), "chat_with_api failed: {:?}", result.err());
        // ... rest of assertions ...
    }
}
