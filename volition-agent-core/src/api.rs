// volition-agent-core/src/api.rs

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::{error::Category as SerdeJsonCategory, json, to_value, Value};
use tokio::time::Duration;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::config::{ModelConfig, RuntimeConfig};
use crate::models::chat::{ApiResponse, ResponseMessage};
use crate::models::tools::ToolDefinition;

/// Sends completion requests to the configured AI model endpoint with retry logic.
/// Renamed from get_chat_completion_from_endpoint, merged higher-level function.
pub async fn get_chat_completion(
    client: &Client,
    config: &RuntimeConfig,
    messages: Vec<ResponseMessage>,
    tool_definitions: &[ToolDefinition],
) -> Result<ApiResponse> {
    // Select model config using the new method on RuntimeConfig
    let model_config = config.selected_model_config()?;
    let url_str = &model_config.endpoint;

    // Moved retry parameters inside
    const MAX_RETRIES: u32 = 5;
    const INITIAL_DELAY: Duration = Duration::from_secs(1);
    const MAX_DELAY: Duration = Duration::from_secs(60);

    let request_body = build_openai_request(
        &model_config.model_name,
        messages,
        model_config,
        tool_definitions,
    )?;

    debug!(
        "Request URL: {}\nRequest JSON: {}",
        url_str,
        serde_json::to_string_pretty(&request_body)?
    );

    let backoff_factor = 2.0;
    let mut retries = 0;
    let mut current_delay = INITIAL_DELAY;

    loop {
        let request = client
            .post(url_str)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", config.api_key));

        let response_result = request.json(&request_body).send().await;

        let response = match response_result {
            Ok(resp) => resp,
            Err(e) => {
                if retries < MAX_RETRIES {
                    retries += 1;
                    warn!(
                        "Network error sending request: {}. Retrying in {:?} (attempt {}/{})",
                        e, current_delay, retries, MAX_RETRIES
                    );
                    tokio::time::sleep(current_delay).await;
                    current_delay = std::cmp::min(
                        Duration::from_secs_f64(current_delay.as_secs_f64() * backoff_factor),
                        MAX_DELAY,
                    );
                    continue;
                } else {
                    return Err(anyhow!("Network error after {} retries: {}", MAX_RETRIES, e));
                }
            }
        };

        let status = response.status();

        if (status.as_u16() == 429 || status.is_server_error()) && retries < MAX_RETRIES {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs);

            let wait_time = retry_after.unwrap_or(current_delay);
            retries += 1;
            warn!(
                "API request failed with status {}. Retrying in {:?} (attempt {}/{})",
                status, wait_time, retries, MAX_RETRIES
            );
            tokio::time::sleep(wait_time).await;
            current_delay = std::cmp::min(
                Duration::from_secs_f64(current_delay.as_secs_f64() * backoff_factor),
                MAX_DELAY,
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
            return Err(anyhow!("API response was not a JSON object: {:?}", response_value));
        };

        if !response_json_obj.contains_key("id") {
            let new_id = format!("chatcmpl-{}", Uuid::new_v4());
            debug!("Added missing 'id' field to API response with value: {}", new_id);
            response_json_obj.insert("id".to_string(), json!(new_id));
        }

        let api_response_result: Result<ApiResponse, serde_json::Error> =
            serde_json::from_value(Value::Object(response_json_obj.clone()));

        let api_response = match api_response_result {
            Ok(resp) => resp,
            Err(e) => {
                if e.classify() == SerdeJsonCategory::Data && e.to_string().contains("missing field `choices`") {
                    warn!(
                        "API response successfully received but missing 'choices' field. Response body: {}",
                        serde_json::to_string_pretty(&response_json_obj).unwrap_or_else(|_| format!("{:?}", response_json_obj))
                    );
                    return Err(anyhow!("API call succeeded but response was missing the expected 'choices' field.").context(e));
                } else {
                    return Err(anyhow!("Failed to deserialize API response").context(e));
                }
            }
        };

        debug!("=== API RESPONSE ===");
        if let Some(choice) = api_response.choices.first() {
            if let Some(tool_calls) = &choice.message.tool_calls {
                debug!("Tool calls: {:#?}", tool_calls);
            } else {
                debug!("No tool calls");
            }
        } else {
            debug!("Response has empty 'choices' array");
        }
        debug!("=====================");

        return Ok(api_response);
    }
}

// build_openai_request remains the same
fn build_openai_request(
    model_name: &str,
    messages: Vec<ResponseMessage>,
    model_config: &ModelConfig,
    tool_definitions: &[ToolDefinition],
) -> Result<Value> {
    let mut request_map = serde_json::Map::new();
    request_map.insert("model".to_string(), json!(model_name));
    request_map.insert("messages".to_string(), to_value(messages)?);

    let tools_json: Vec<Value> = tool_definitions
        .iter()
        .map(|tool_def| {
            json!({
                "type": "function",
                "function": tool_def
            })
        })
        .collect();

    if !tools_json.is_empty() {
        request_map.insert("tools".to_string(), Value::Array(tools_json));
    }

    if let Some(parameters) = model_config.parameters.as_table() {
        for (key, value) in parameters {
            let json_value = to_value(value.clone())
                .with_context(|| format!("Failed to convert TOML parameter '{}' to JSON", key))?;
            request_map.insert(key.clone(), json_value);
        }
    }
    Ok(Value::Object(request_map))
}

// Original get_chat_completion function is now removed.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ModelConfig, RuntimeConfig};
    use crate::models::chat::ResponseMessage;
    use crate::models::tools::{ToolDefinition, ToolParametersDefinition, ToolParameter, ToolParameterType};
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;
    use toml;

    use httpmock::prelude::*;
    use tokio;

    // --- Test Helpers ---
    fn create_mock_tool_definitions() -> Vec<ToolDefinition> {
        let mut properties = HashMap::new();
        properties.insert("arg1".to_string(), ToolParameter {
             param_type: ToolParameterType::String, description: "Arg 1".to_string(), enum_values: None, items: None
        });
        vec![ToolDefinition {
            name: "mock_tool".to_string(), description: "A mock tool".to_string(),
            parameters: ToolParametersDefinition { param_type: "object".to_string(), properties, required: vec!["arg1".to_string()] },
        }]
    }

    fn create_test_model_config(endpoint: &str, params: Option<toml::value::Table>) -> ModelConfig {
        ModelConfig {
            model_name: "test-model-name".to_string(),
            endpoint: endpoint.to_string(),
            parameters: params.map(toml::Value::Table).unwrap_or_default(),
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

    // Test retry parameters are now defined within get_chat_completion,
    // but we can use similar values for verifying retry counts in tests.
    const TEST_MAX_RETRIES: u32 = 5;
    // const TEST_INITIAL_DELAY: Duration = Duration::from_millis(10); // No longer passed
    // const TEST_MAX_DELAY: Duration = Duration::from_millis(50); // No longer passed

    // --- Tests for build_openai_request (unchanged) ---
    #[test]
    fn test_build_openai_request_basic() { /* ... */ }
    #[test]
    fn test_build_openai_request_no_tools() { /* ... */ }
    #[test]
    fn test_build_openai_request_with_parameters() { /* ... */ }

    // --- Tests for get_chat_completion (merged function) ---

    #[tokio::test]
    async fn test_get_chat_completion_success() {
        let server = MockServer::start_async().await;
        let model_key = "selected_model";
        let endpoint_path = "/v1/chat/completions";
        let full_endpoint_url = format!("{}{}", server.base_url(), endpoint_path);
        let messages = vec![ResponseMessage { role: "user".into(), content: Some("Ping".into()), ..Default::default() }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let specific_model_config = runtime_config.selected_model_config().unwrap(); // Get selected config
        let tool_definitions = create_mock_tool_definitions();

        let mock = server.mock_async(|when, then| {
            when.method(POST).path(endpoint_path)
                .json_body(build_openai_request(&specific_model_config.model_name, messages.clone(), specific_model_config, &tool_definitions).unwrap());
            then.status(200).json_body(json!({
                "id": "chatcmpl-123", "choices": [{"index": 0, "message": {"role": "assistant", "content": "Pong"}, "finish_reason": "stop"}]
            }));
        }).await;

        let client = Client::new();
        // Call the merged function
        let result = get_chat_completion(
            &client,
            &runtime_config,
            messages,
            &tool_definitions,
        ).await;

        mock.assert_async().await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        let response = result.unwrap();
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.choices[0].message.content, Some("Pong".to_string()));
    }

    #[tokio::test]
    async fn test_get_chat_completion_selects_correct_model() {
        let server = MockServer::start_async().await;
        let endpoint_path_b = "/v1/model_b";
        let server_url = server.base_url();

        let model_config_a = create_test_model_config(&format!("{}/v1/model_a", server_url), None);
        let model_config_b = create_test_model_config(&format!("{}{}", server_url, endpoint_path_b), None);

        let mut models = HashMap::new();
        models.insert("model_a".to_string(), model_config_a);
        models.insert("model_b".to_string(), model_config_b.clone()); // Clone for request building check

        let runtime_config = RuntimeConfig {
            selected_model: "model_b".to_string(), // Select model B
            models,
            ..create_test_runtime_config("dummy", ModelConfig { model_name: "".into(), endpoint: "".into(), parameters: Default::default() })
        };
        let messages = vec![ResponseMessage { role: "user".into(), content: Some("Select B".into()), ..Default::default() }];
        let tool_definitions = create_mock_tool_definitions();

        // Mock expects call to model B's endpoint
        let mock_b = server.mock_async(|when, then| {
            when.method(POST).path(endpoint_path_b)
                .json_body(build_openai_request(&model_config_b.model_name, messages.clone(), &model_config_b, &tool_definitions).unwrap());
            then.status(200).json_body(json!({
                "id": "chatcmpl-selected-b", "choices": [{"index": 0, "message": {"role": "assistant", "content": "Selected B"}, "finish_reason": "stop"}]
            }));
        }).await;

        let client = Client::new();
        let result = get_chat_completion(&client, &runtime_config, messages, &tool_definitions).await;

        mock_b.assert_async().await;
        assert!(result.is_ok(), "get_chat_completion failed: {:?}", result.err());
        assert_eq!(result.unwrap().id, "chatcmpl-selected-b");
    }

    #[tokio::test]
    async fn test_get_chat_completion_retry_and_fail() {
        let server = MockServer::start_async().await;
        let model_key = "retry_model";
        let endpoint_path = "/v1/chat/completions";
        let full_endpoint_url = format!("{}{}", server.base_url(), endpoint_path);
        let messages = vec![ResponseMessage { role: "user".into(), content: Some("Retry".into()), ..Default::default() }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let tool_definitions = create_mock_tool_definitions();

        // Mock server to always return 500
        let mock = server.mock_async(|when, then| {
            when.method(POST).path(endpoint_path);
            then.status(500).body("Server error");
        }).await;

        let client = Client::new();
        // Call the merged function - retry params are internal now
        let result = get_chat_completion(
            &client,
            &runtime_config,
            messages,
            &tool_definitions,
        ).await;

        // Check mock hits based on internal retry count
        assert_eq!(mock.hits(), TEST_MAX_RETRIES as usize + 1);
        assert!(result.is_err(), "Expected Err, got Ok");
        assert!(result.err().unwrap().to_string().contains("API error: 500"));
    }

    // Other tests (401, missing choices) can be adapted similarly by calling the merged get_chat_completion
}
