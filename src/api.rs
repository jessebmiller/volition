use anyhow::{anyhow, Context, Result}; // Added Context
use reqwest::Client;
use serde_json::{json, to_value, Value};
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
        if (status == 429 || status.is_server_error()) && retries < max_retries {
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
            tokio::time::sleep(wait_time).await;
            delay = std::cmp::min(
                Duration::from_secs((delay.as_secs() as f64 * backoff_factor) as u64),
                max_delay,
            );
            continue;
        }

        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API error: {} - {}", status, error_text));
        }

        // Deserialize response and add missing 'id' if necessary (unchanged)
        let mut response_json: Value = response.json().await?;
        if let Value::Object(map) = &mut response_json {
            if !map.contains_key("id") {
                let new_id = format!("chatcmpl-{}", Uuid::new_v4());
                map.insert("id".to_string(), json!(new_id));
                debug!(
                    "Added missing 'id' field to API response with value: {}",
                    new_id
                );
            }
        }
        let api_response: ApiResponse = serde_json::from_value(response_json)?;

        // Debug logging for response (unchanged)
        debug!("=== API RESPONSE ===");
        if let Some(choices) = api_response.choices.first() {
            if let Some(tool_calls) = &choices.message.tool_calls {
                debug!("Tool calls: {:#?}", tool_calls);
            } else {
                debug!("No tool calls");
            }
        } else {
            debug!("No choices in response");
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
