use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, to_value, Value};
use std::collections::HashMap;
use tokio::time::Duration;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::models::chat::{ApiResponse, ResponseMessage};
use crate::models::tools::Tools;
// Use the combined RuntimeConfig and ModelConfig
use crate::config::{RuntimeConfig, ModelConfig};

/// Unified function to send chat requests to various endpoints.
/// It constructs the proper URL, request body, and headers based on the service type and any endpoint override provided in ModelConfig.
pub async fn chat_with_endpoint(
    client: &Client,
    api_key: Option<&str>, // The API key is passed as an option
    model_config: &ModelConfig, // Use the specific model config
    messages: Vec<ResponseMessage>,
) -> Result<ApiResponse> {
    // Determine the URL: use endpoint_override if set, otherwise use defaults per service.
    let url = if let Some(endpoint) = &model_config.endpoint_override {
        endpoint.clone()
    } else {
        // Use case-insensitive matching for service name
        match model_config.service.to_lowercase().as_str() {
            "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
            "ollama" => "http://localhost:11434/v1/chat/completions".to_string(),
            other => return Err(anyhow!("Unsupported service in model config: {}", other)),
        }
    };

    // Build the request body based on the service type specified in the model config
    let request_body = match model_config.service.to_lowercase().as_str() {
        "openai" | "ollama" => build_openai_request(&model_config.model_name, messages, model_config)?,
        other => return Err(anyhow!("Unsupported service in model config: {}", other)),
    };

    debug!("Request URL: {}\nRequest JSON: {}", url, serde_json::to_string_pretty(&request_body)?);

    // Exponential backoff parameters
    let max_retries = 5;
    let initial_delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(60);
    let backoff_factor = 2.0;

    let mut retries = 0;
    let mut delay = initial_delay;

    loop {
        let mut request = client.post(&url).header("Content-Type", "application/json");

        // Add service-specific headers and authentication based on model_config.service
        match model_config.service.to_lowercase().as_str() {
            "openai" => {
                // All OpenAI-compatible services use Bearer token authentication.
                if let Some(key) = api_key {
                    request = request.header("Authorization", format!("Bearer {}", key));
                } else {
                    // Only error if it's strictly OpenAI service (default endpoint) and key is missing
                    if model_config.service.eq_ignore_ascii_case("openai") && model_config.endpoint_override.is_none() {
                         return Err(anyhow!("API key is required for the default OpenAI service endpoint"));
                    }
                    // If using an override or a non-OpenAI service via this path, a missing key might be acceptable.
                    warn!("No API key provided for OpenAI-compatible service at {}", url);
                }
            },
            "ollama" => {
                // Ollama doesn't typically require auth by default
            }
            _ => {}
        }

        let response_result = request.json(&request_body).send().await;

        let response = match response_result {
            Ok(resp) => resp,
            Err(e) => {
                 // Handle network errors with retry
                 if retries < max_retries {
                    retries += 1;
                    warn!("Network error sending request: {}. Retrying in {} seconds (attempt {}/{})", e, delay.as_secs(), retries, max_retries);
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(Duration::from_secs((delay.as_secs() as f64 * backoff_factor) as u64), max_delay);
                    continue;
                 } else {
                    return Err(anyhow!("Network error after {} retries: {}", max_retries, e));
                 }
            }
        };

        let status = response.status();

        // Handle rate limiting and server errors with retry mechanism
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
            warn!("API request failed with status {}. Retrying in {} seconds (attempt {}/{})", status, wait_time.as_secs(), retries, max_retries);
            tokio::time::sleep(wait_time).await;
            delay = std::cmp::min(Duration::from_secs((delay.as_secs() as f64 * backoff_factor) as u64), max_delay);
            continue;
        }

        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API error: {} - {}", status, error_text));
        }

        // Deserialize the response into a generic JSON Value first
        let mut response_json: Value = response.json().await?;

        // Check if the response is an object and if the 'id' field is missing
        if let Value::Object(map) = &mut response_json {
            if !map.contains_key("id") {
                // Generate a new UUID v4 and format it like OpenAI's IDs
                let new_id = format!("chatcmpl-{}", Uuid::new_v4());
                // Insert the new ID into the JSON map
                map.insert("id".to_string(), json!(new_id));
                debug!("Added missing 'id' field to API response with value: {}", new_id);
            }
        }

        // Now deserialize the potentially modified JSON Value into the ApiResponse struct
        let api_response: ApiResponse = serde_json::from_value(response_json)?;

        debug!("=== API RESPONSE ===");
        if let Some(choices) = api_response.choices.get(0) {
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

// This function remains largely the same, using ModelConfig
fn build_openai_request(
    model_name: &str,
    messages: Vec<ResponseMessage>,
    model_config: &ModelConfig,
) -> Result<Value> {
    let mut request_map = serde_json::Map::new();
    request_map.insert("model".to_string(), json!(model_name));
    request_map.insert("messages".to_string(), to_value(messages)?);
    
    // Add tools based on model_config service type (case-insensitive)
    if model_config.service.eq_ignore_ascii_case("openai") || model_config.endpoint_override.is_some() {
        request_map.insert("tools".to_string(), json!([
            Tools::shell_definition(),
            Tools::read_file_definition(),
            Tools::write_file_definition(),
            Tools::search_text_definition(),
            Tools::find_definition_definition(),
            Tools::user_input_definition()
        ]));
    }

    // Add parameters from model_config.parameters
    if let Some(parameters) = model_config.parameters.as_table() {
        for (key, value) in parameters {
            let json_value = to_value(value.clone())?;
            request_map.insert(key.clone(), json_value);
        }
    }

    Ok(Value::Object(request_map))
}

/// This function selects the appropriate service based on RuntimeConfig and delegates the API call.
pub async fn chat_with_api(
    client: &Client,
    config: &RuntimeConfig, // Use the combined RuntimeConfig
    messages: Vec<ResponseMessage>,
    overrides: Option<HashMap<String, String>>, // Overrides might need adjustment or removal depending on use case
) -> Result<ApiResponse> {
    // Clone the config to apply potential overrides (if overrides are kept)
    let mut effective_config = config.clone();

    // Apply overrides - Note: Overriding parts of RuntimeConfig might be complex.
    // Consider if overrides are still needed or how they should apply to the new structure.
    // For now, keeping the logic but it might need refinement.
    if let Some(overrides) = overrides {
        warn!("Applying overrides to RuntimeConfig. Ensure override keys match the new structure.");
        for (key, value) in overrides {
            match key.as_str() {
                "api_key" => effective_config.api_key = value,
                "selected_model" => {
                    // This override assumes the active service has a selected_model field (like openai)
                    match effective_config.active_service.service.to_lowercase().as_str() {
                        "openai" => effective_config.openai.selected_model = value,
                        _ => debug!("Override 'selected_model' ignored for active service: {}", effective_config.active_service.service),
                    }
                },
                "active_service" => {
                    // Clone the value for the assignment
                    effective_config.active_service.service = value.clone();
                    // Warning: Changing active_service via override might lead to inconsistencies
                    // if not carefully managed with selected_model overrides.
                    warn!("Overriding active_service to '{}'. Ensure selected_model is compatible.", value);
                },
                 // Add overrides for system_prompt or other fields if needed
                "system_prompt" => {
                    effective_config.system_prompt = value;
                    debug!("Overriding system_prompt.");
                }
                _ => debug!("Unknown config override key: {}", key),
            }
        }
        // Re-validate potentially modified config? Or assume overrides are valid.
    }

    // Use the active service from the potentially overridden config
    let active_service = &effective_config.active_service.service;
    
    // Determine the selected model name based on the active service section
    let selected_model_name = match active_service.to_lowercase().as_str() {
        "openai" => &effective_config.openai.selected_model,
        // Add other cases if services have their own selected_model field
        _ => {
             return Err(anyhow!(
                "Active service '{}' is specified, but its configuration section (e.g., [{}]) with a 'selected_model' field is missing or the service is not supported for model selection this way.",
                active_service, active_service
            ));
        }
    };

    // Retrieve the configuration for the selected model from the potentially overridden config
    let model_config = effective_config.models.get(selected_model_name)
        .ok_or_else(|| anyhow!("Configuration for selected model '{}' not found in [models] section", selected_model_name))?;

    // Final check: Ensure the retrieved model_config's service matches the active_service (case-insensitive)
    if !model_config.service.eq_ignore_ascii_case(active_service) {
        return Err(anyhow!(
            "Configuration mismatch: Active service is '{}' but the selected model '{}' belongs to service '{}'",
            active_service,
            selected_model_name,
            model_config.service
        ));
    }

    // Pass the API key from the potentially overridden config
    let api_key_option = Some(effective_config.api_key.as_str());

    // Call the unified endpoint function with the specific model_config
    chat_with_endpoint(client, api_key_option, model_config, messages).await
}
