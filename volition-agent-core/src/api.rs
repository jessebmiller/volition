// volition-agent-core/src/api.rs

use anyhow::{anyhow, Context, Result};
use reqwest::Client; // Import HeaderMap if needed for cloning
use serde_json::{json, to_value, Value};
use tokio::time::Duration;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::config::{ModelConfig, RuntimeConfig};
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::models::tools::ToolDefinition;

pub async fn get_chat_completion(
    client: &Client,
    config: &RuntimeConfig,
    messages: Vec<ChatMessage>,
    tool_definitions: &[ToolDefinition],
) -> Result<ApiResponse> {
    let model_config = config.selected_model_config()?;
    let url_str = &model_config.endpoint;

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
                    return Err(anyhow!(
                        "Network error after {} retries: {}",
                        MAX_RETRIES,
                        e
                    ));
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
            // Clone headers before consuming the response body
            let headers = response.headers().clone();
            // Read the response body text
            let error_text = response
                .text()
                .await
                .context("Failed to read API error response body")?;
            // New detailed debug log with status, headers, and body
            debug!(
                "API request failed. Status: {}, Headers: {:#?}, Body: {}",
                status,
                headers, // Use the cloned headers
                error_text
            );
            // Return the error including the status and body text
            return Err(anyhow!("API error: {} - {}", status, error_text));
        }

        let response_value: Value = response
            .json()
            .await
            .context("Failed to read API response body as JSON")?;

        let mut response_json_obj = if let Value::Object(map) = response_value.clone() {
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
                debug!(
                    "ERROR: failed to deserialize API response {:#?}",
                    response_value.clone()
                );
                return Err(anyhow!("Failed to deserialize API response").context(e));
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

fn build_openai_request(
    model_name: &str,
    messages: Vec<ChatMessage>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ModelConfig, RuntimeConfig};
    use crate::models::chat::ChatMessage;
    use crate::models::tools::{
        ToolDefinition, ToolParameter, ToolParameterType, ToolParametersDefinition,
    };
    use serde_json::json;
    use std::collections::HashMap;
    // use std::path::PathBuf; // Removed unused import
    // use std::time::Duration; // Removed unused import
    use toml;

    use httpmock::prelude::*;
    use tokio;

    // --- Test Helpers ---
    fn create_mock_tool_definitions() -> Vec<ToolDefinition> {
        let mut properties = HashMap::new();
        properties.insert(
            "arg1".to_string(),
            ToolParameter {
                param_type: ToolParameterType::String,
                description: "Arg 1".to_string(),
                enum_values: None,
                items: None,
            },
        );
        vec![ToolDefinition {
            name: "mock_tool".to_string(),
            description: "A mock tool".to_string(),
            parameters: ToolParametersDefinition {
                param_type: "object".to_string(),
                properties,
                required: vec!["arg1".to_string()],
            },
        }]
    }

    fn create_test_model_config(endpoint: &str, params: Option<toml::value::Table>) -> ModelConfig {
        ModelConfig {
            model_name: "test-model-name".to_string(),
            endpoint: endpoint.to_string(),
            parameters: params.map_or(toml::Value::Table(Default::default()), toml::Value::Table),
        }
    }

    // Updated test helper: Removed project_root
    fn create_test_runtime_config(selected_key: &str, model_config: ModelConfig) -> RuntimeConfig {
        let mut models = HashMap::new();
        models.insert(selected_key.to_string(), model_config);
        RuntimeConfig {
            system_prompt: "Test prompt".to_string(),
            selected_model: selected_key.to_string(),
            models,
            api_key: "default-test-api-key".to_string(),
            // project_root: PathBuf::from("/fake/path"), // Removed field
        }
    }

    const TEST_MAX_RETRIES: u32 = 5;

    // --- Tests for build_openai_request ---
    #[test]
    fn test_build_openai_request_basic() {
        let model_name = "gpt-basic";
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: Some("Hello".into()),
            ..Default::default()
        }];
        let model_config = create_test_model_config("http://fake.endpoint/v1", None);
        let tool_definitions = create_mock_tool_definitions();
        let result = build_openai_request(
            model_name,
            messages.clone(),
            &model_config,
            &tool_definitions,
        );
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["messages"], json!(messages));
    }
    #[test]
    fn test_build_openai_request_no_tools() {
        let model_name = "gpt-no-tools";
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: Some("Hi".into()),
            ..Default::default()
        }];
        let model_config = create_test_model_config("http://fake.endpoint/v1", None);
        let tool_definitions: Vec<ToolDefinition> = vec![];
        let result = build_openai_request(
            model_name,
            messages.clone(),
            &model_config,
            &tool_definitions,
        );
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["messages"], json!(messages));
    }
    #[test]
    fn test_build_openai_request_with_parameters() {
        let model_name = "gpt-params";
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: Some("Test".into()),
            ..Default::default()
        }];
        let mut params = toml::value::Table::new();
        params.insert("temperature".to_string(), toml::Value::Float(0.9));
        let model_config = create_test_model_config("http://fake.endpoint/v1", Some(params));
        let tool_definitions = create_mock_tool_definitions();
        let result = build_openai_request(
            model_name,
            messages.clone(),
            &model_config,
            &tool_definitions,
        );
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["messages"], json!(messages));
    }

    // --- Tests for get_chat_completion ---
    #[tokio::test]
    async fn test_get_chat_completion_success() {
        let server = MockServer::start_async().await;
        let model_key = "selected_model";
        let endpoint_path = "/v1/chat/completions";
        let full_endpoint_url = format!("{}{}", server.base_url(), endpoint_path);
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: Some("Ping".into()),
            ..Default::default()
        }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let specific_model_config = runtime_config.selected_model_config().unwrap();
        let tool_definitions = create_mock_tool_definitions();

        let mock = server
            .mock_async(|when, then| {
                when.method(POST).path(endpoint_path).json_body(
                    build_openai_request(
                        &specific_model_config.model_name,
                        messages.clone(),
                        specific_model_config,
                        &tool_definitions,
                    )
                    .unwrap(),
                );
                then.status(200).json_body(json!({
                    "id": "chatcmpl-123", "choices": [{"index": 0, "message": {"role": "assistant", "content": "Pong"}, "finish_reason": "stop"}]
                }));
            })
            .await;

        let client = Client::new();
        let result =
            get_chat_completion(&client, &runtime_config, messages, &tool_definitions).await;
        mock.assert_async().await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        assert_eq!(result.unwrap().id, "chatcmpl-123");
    }

    #[tokio::test]
    async fn test_get_chat_completion_selects_correct_model() {
        let server = MockServer::start_async().await;
        let endpoint_path_b = "/v1/model_b";
        let server_url = server.base_url();
        let model_config_a =
            create_test_model_config(&format!("{}{}", server_url, "/v1/model_a"), None);
        let model_config_b =
            create_test_model_config(&format!("{}{}", server_url, endpoint_path_b), None);
        let mut models = HashMap::new();
        models.insert("model_a".to_string(), model_config_a);
        models.insert("model_b".to_string(), model_config_b.clone());
        // Updated dummy config creation
        let dummy_model_config = ModelConfig {
            model_name: "".into(),
            endpoint: "".into(),
            parameters: toml::Value::Table(Default::default()),
        };
        let runtime_config = RuntimeConfig {
            selected_model: "model_b".to_string(),
            models,
            ..create_test_runtime_config("dummy", dummy_model_config)
        };
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: Some("Select B".into()),
            ..Default::default()
        }];
        let tool_definitions = create_mock_tool_definitions();

        let mock_b = server
            .mock_async(|when, then| {
                when.method(POST).path(endpoint_path_b).json_body(
                    build_openai_request(
                        &model_config_b.model_name,
                        messages.clone(),
                        &model_config_b,
                        &tool_definitions,
                    )
                    .unwrap(),
                );
                then.status(200).json_body(json!({
                    "id": "chatcmpl-selected-b", "choices": [{"index": 0, "message": {"role": "assistant", "content": "Selected B"}, "finish_reason": "stop"}]
                }));
            })
            .await;

        let client = Client::new();
        let result =
            get_chat_completion(&client, &runtime_config, messages, &tool_definitions).await;
        mock_b.assert_async().await;
        assert!(
            result.is_ok(),
            "get_chat_completion failed: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().id, "chatcmpl-selected-b");
    }

    #[tokio::test]
    #[ignore = "Waits for full retry duration (~30s+)"]
    async fn test_get_chat_completion_retry_and_fail() {
        let server = MockServer::start_async().await;
        let model_key = "retry_model";
        let endpoint_path = "/v1/chat/completions";
        let full_endpoint_url = format!("{}{}", server.base_url(), endpoint_path);
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: Some("Retry".into()),
            ..Default::default()
        }];
        let model_config = create_test_model_config(&full_endpoint_url, None);
        let runtime_config = create_test_runtime_config(model_key, model_config.clone());
        let tool_definitions = create_mock_tool_definitions();

        let mock = server
            .mock_async(|when, then| {
                when.method(POST).path(endpoint_path);
                then.status(500).body("Server error");
            })
            .await;

        let client = Client::new();
        let result =
            get_chat_completion(&client, &runtime_config, messages, &tool_definitions).await;
        assert_eq!(mock.hits(), TEST_MAX_RETRIES as usize + 1);
        assert!(result.is_err(), "Expected Err, got Ok");
        assert!(result.err().unwrap().to_string().contains("API error: 500"));
    }
}
