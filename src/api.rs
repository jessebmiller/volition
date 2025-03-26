use anyhow::{anyhow, Result};
use colored::*;
use reqwest::Client;
use serde_json::{json, to_value, Value};
use std::collections::HashMap;
use tokio::time::Duration;
use tracing::{debug, info, warn};

use crate::models::chat::{ApiResponse, ResponseMessage};
use crate::models::tools::Tools;
use crate::config::{Config, ModelConfig};


/// Unified function to send chat requests to various endpoints.
/// It constructs the proper URL, request body, and headers based on the service type and any endpoint override provided in ModelConfig.
pub async fn chat_with_endpoint(
    client: &Client,
    api_key: Option<&str>,
    model_config: &ModelConfig,
    messages: Vec<ResponseMessage>,
) -> Result<ApiResponse> {
    // Determine the URL: use endpoint_override if set, otherwise use defaults per service.
    let url = if let Some(endpoint) = &model_config.endpoint_override {
        endpoint.clone()
    } else {
        match model_config.service.as_str() {
            "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
            "ollama" => "http://localhost:11434/v1/chat/completions".to_string(),
            other => return Err(anyhow!("Unsupported service: {}", other)),
        }
    };

    // Build the request body based on the service type.
    // For OpenAI, include the model, messages, tools, and any additional parameters.
    // For other services (e.g., Ollama), a simpler request body might suffice.
    let mut request_body: Value = if model_config.service == "openai" {
        json!({
            "model": model_config.model_name,
            "messages": messages,
            "tools": [
                Tools::shell_definition(),
                Tools::read_file_definition(),
                Tools::write_file_definition(),
                Tools::search_code_definition(),
                Tools::find_definition_definition(),
                Tools::user_input_definition()
            ]
        })
    } else {
        // For other services, we currently just forward the messages. Additional customization can be added here.
        json!({
            "messages": messages
        })
    };

    // Merge additional parameters if present (only applicable for OpenAI-like endpoints where extra parameters are allowed)
    if model_config.service == "openai" {
        if let Some(parameters) = model_config.parameters.as_table() {
            for (key, value) in parameters {
                let json_value = to_value(value.clone())?;
                request_body[key] = json_value;
            }
        }
    }

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

        // Add Authorization header if service is OpenAI and an API key is provided
        if model_config.service == "openai" {
            if let Some(key) = api_key {
                request = request.header("Authorization", format!("Bearer {}", key));
            } else {
                return Err(anyhow!("API key required for OpenAI service"));
            }
        }

        let response = request.json(&request_body).send().await?;
        let status = response.status();

        // Handle rate limiting and server errors with retry mechanism
        if (status == 429 || status.as_u16() >= 500) && retries < max_retries {
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
            info!("API request failed with status {}. Retrying in {} seconds (attempt {}/{})", status, wait_time.as_secs(), retries, max_retries);
            warn!("Rate limited by API. Retrying in {} seconds (attempt {}/{})", wait_time.as_secs(), retries, max_retries);
            tokio::time::sleep(wait_time).await;
            delay = std::cmp::min(Duration::from_secs((delay.as_secs() as f64 * backoff_factor) as u64), max_delay);
            continue;
        }

        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API error: {} - {}", status, error_text));
        }

        let api_response: ApiResponse = response.json().await?;

        debug!("=== API RESPONSE ===");
        if let Some(tool_calls) = &api_response.choices[0].message.tool_calls {
            // Log tool calls at debug level with detailed information
            debug!("Tool calls: {:#?}", tool_calls);
        } else {
            debug!("No tool calls");
        }
        debug!("=====================");

        return Ok(api_response);
    }
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
                "openai_api_key" => effective_config.openai.api_key = value,
                "selected_model" => effective_config.openai.selected_model = value,
                _ => debug!("Unknown config override: {}", key),
            }
        }
    }

    // Select the model configuration based on the selected model
    let model_config = effective_config.models.get(&effective_config.openai.selected_model)
        .ok_or_else(|| anyhow!("Unsupported model: {}", effective_config.openai.selected_model))?;

    // Determine the API key to use if required
    let api_key_option = if model_config.service == "openai" {
        Some(effective_config.openai.api_key.as_str())
    } else {
        None
    };

    chat_with_endpoint(client, api_key_option, model_config, messages).await
}
