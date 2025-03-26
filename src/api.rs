use anyhow::{anyhow, Result};
use reqwest::Client;
// Removed unused import: Map
use serde_json::{json, to_value, Value};
use std::collections::HashMap;
use tokio::time::Duration;
use tracing::{debug, warn}; // Removed unused import: info
use uuid::Uuid; // Import Uuid

use crate::models::chat::{ApiResponse, ResponseMessage};
use crate::models::tools::Tools;
use crate::config::{Config, ModelConfig};


/// Unified function to send chat requests to various endpoints.
/// It constructs the proper URL, request body, and headers based on the service type and any endpoint override provided in ModelConfig.
pub async fn chat_with_endpoint(
    client: &Client,
    api_key: Option<&str>, // The API key is passed as an option
    model_config: &ModelConfig,
    messages: Vec<ResponseMessage>,
) -> Result<ApiResponse> {
    // Determine the URL: use endpoint_override if set, otherwise use defaults per service.
    let url = if let Some(endpoint) = &model_config.endpoint_override {
        endpoint.clone()
    } else {
        match model_config.service.as_str() {
            // Default URL for OpenAI compatible services.
            "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
            // Removed "gemini" default URL case
            "ollama" => "http://localhost:11434/v1/chat/completions".to_string(),
            other => return Err(anyhow!("Unsupported service: {}", other)),
        }
    };

    // Build the request body based on the service type.
    // Since we're using the OpenAI-compatible endpoint, we always use build_openai_request.
    let request_body = match model_config.service.as_str() {
        "openai" | "ollama" => build_openai_request(&model_config.model_name, messages, model_config)?,
        // Removed "gemini" request building case
        other => return Err(anyhow!("Unsupported service: {}", other)),
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

        // Add service-specific headers and authentication
        match model_config.service.as_str() {
            "openai" => {
                // All OpenAI-compatible services use Bearer token authentication.
                if let Some(key) = api_key {
                    request = request.header("Authorization", format!("Bearer {}", key));
                } else {
                    // Only error if it's strictly OpenAI service and key is missing
                    // For other compatible services using this path, key might be optional or handled differently (e.g., Ollama)
                    if model_config.service == "openai" && model_config.endpoint_override.is_none() { // Be more specific: only error if using default OpenAI URL
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
                // Log tool calls at debug level with detailed information
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

fn build_openai_request(
    model_name: &str,
    messages: Vec<ResponseMessage>,
    model_config: &ModelConfig,
) -> Result<Value> {
    let mut request_map = serde_json::Map::new();
    request_map.insert("model".to_string(), json!(model_name));
    request_map.insert("messages".to_string(), to_value(messages)?);
    
    // Add tools only if the service is 'openai' or if an endpoint_override is present (assuming compatibility)
    if model_config.service == "openai" || model_config.endpoint_override.is_some() {
        request_map.insert("tools".to_string(), json!([
            Tools::shell_definition(),
            Tools::read_file_definition(),
            Tools::write_file_definition(),
            Tools::search_code_definition(),
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

/// This function selects the appropriate service based on configuration and delegates the API call to the unified chat_with_endpoint function.
pub async fn chat_with_api(
    client: &Client,
    config: &Config,
    messages: Vec<ResponseMessage>,
    overrides: Option<HashMap<String, String>>,
) -> Result<ApiResponse> {
    // Create a clone of the config to modify
    let mut effective_config = config.clone();

    // Apply overrides to the configuration
    if let Some(overrides) = overrides {
        for (key, value) in overrides {
            match key.as_str() {
                "api_key" => effective_config.api_key = value, // Updated override key
                "selected_model" => {
                    // Update the selected model based on the active service
                    match effective_config.active_service.service.as_str() {
                        "openai" => effective_config.openai.selected_model = value,
                        // Add cases for other services if their selected_model field differs
                        _ => debug!("Override 'selected_model' ignored for active service: {}", effective_config.active_service.service),
                    }
                },
                "active_service" => {
                    effective_config.active_service.service = value;
                },
                _ => debug!("Unknown config override: {}", key),
            }
        }
    }

    // Use the active service from configuration
    let active_service = &effective_config.active_service.service;
    
    // Determine the selected model name based on the active service
    let selected_model_name = match active_service.as_str() {
        "openai" => &effective_config.openai.selected_model,
        // For services like ollama, if there isn't a specific selected_model field,
        // we might need to find a default or the first one listed for that service.
        // For now, assume a structure similar to openai or that the model name is directly usable.
        // If adding more distinct services, this logic might need refinement.
        "ollama" => {
             // Attempt to find a model name. This might need a dedicated field like `ollama.selected_model` in Config for clarity.
             // For now, let's rely on the validation in load_config ensuring *a* model exists for the service.
             // We'll just use the model name found via the models map lookup later.
             // Placeholder: If ollama had a specific selected_model field: &effective_config.ollama.selected_model,
             // Since it doesn't, we proceed and rely on the models map lookup.
             effective_config.models.iter()
                .find(|(_, model_cfg)| model_cfg.service == "ollama")
                .map(|(name, _)| name.as_str())
                .ok_or_else(|| anyhow!("No model configured for the 'ollama' service in config.toml"))?
        }
        _ => return Err(anyhow!("Unsupported active service: {:?}", active_service)),
    };

    // Retrieve the configuration for the selected model
    let model_config = effective_config.models.get(selected_model_name)
        .ok_or_else(|| anyhow!("Configuration for model '{}' not found", selected_model_name))?;

    // Check if the service of the selected model matches the active service (redundant with load_config validation, but safe)
    if model_config.service.to_lowercase() != *active_service {
        return Err(anyhow!(
            "Mismatch between active service '{}' and selected model '{}' service '{}'",
            active_service,
            selected_model_name,
            model_config.service
        ));
    }

    // Pass the API key from the effective_config (which includes env var value and potential override)
    let api_key_option = Some(effective_config.api_key.as_str());

    // Call the unified endpoint function
    chat_with_endpoint(client, api_key_option, model_config, messages).await
}
