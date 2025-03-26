use anyhow::{anyhow, Result};
use colored::*;
use reqwest::Client;
use serde_json::{json, to_value};
use std::collections::HashMap;
use tokio::time::Duration;

use crate::models::chat::{ApiResponse, ResponseMessage};
use crate::models::tools::Tools;
use crate::utils::DebugLevel;
use crate::utils::debug_log;
use crate::config::{Config, ModelConfig};

pub async fn chat_with_api(
    client: &Client,
    config: &Config,
    messages: Vec<ResponseMessage>,
    debug_level: DebugLevel,
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
                _ => debug_log(debug_level, DebugLevel::Minimal, &format!("Unknown config override: {}", key)),
            }
        }
    }

    // Select the model configuration based on the selected model
    let model_config = effective_config.models.get(&effective_config.openai.selected_model)
        .ok_or_else(|| anyhow!("Unsupported model: {}", effective_config.openai.selected_model))?;

    match model_config.service.as_str() {
        "openai" => chat_with_openai(client, &effective_config.openai.api_key, model_config, messages, debug_level).await,
        "ollama" => chat_with_ollama(client, &model_config.service, messages, debug_level).await,
        _ => Err(anyhow!("Unsupported service: {}", model_config.service)),
    }
}

pub async fn chat_with_openai(
    client: &Client,
    api_key: &str,
    model_config: &ModelConfig,
    messages: Vec<ResponseMessage>,
    debug_level: DebugLevel,
) -> Result<ApiResponse> {
    debug_log(debug_level, DebugLevel::Minimal, "\n=== SENDING MESSAGES TO OPENAI API ===");

    if debug_level >= DebugLevel::Minimal {
        for (i, msg) in messages.iter().enumerate() {
            debug_log(
                debug_level,
                DebugLevel::Minimal,
                &format!(
                    "[{}] role: {}, tool_call_id: {:?}, content length: {}",
                    i,
                    msg.role,
                    msg.tool_call_id,
                    msg.content.as_ref().map_or(0, |c| c.len())
                )
            );
        }
    }

    let url = "https://api.openai.com/v1/chat/completions";

    let mut request_body = json!({
        "model": model_config.service,
        "messages": messages,
        "tools": [
            Tools::shell_definition(),
            Tools::read_file_definition(),
            Tools::write_file_definition(),
            Tools::search_code_definition(),
            Tools::find_definition_definition(),
            Tools::user_input_definition()
        ]
    });

    // Add model-specific parameters
    if let Some(parameters) = model_config.parameters.as_table() {
        for (key, value) in parameters {
            // Convert toml::Value to serde_json::Value at the point of assignment
            let json_value = to_value(value.clone())?;
            request_body[key] = json_value;
        }
    }

    if debug_level >= DebugLevel::Verbose {
        debug_log(
            debug_level,
            DebugLevel::Verbose,
            &format!("Request JSON: {}", serde_json::to_string_pretty(&request_body)?)
        );
    }

    // Exponential backoff parameters
    let max_retries = 5;
    let initial_delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(60);
    let backoff_factor = 2.0;

    // Retry loop with exponential backoff
    let mut retries = 0;
    let mut delay = initial_delay;

    loop {
        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();

        // Handle rate limiting and server errors
        if (status == 429 || status.as_u16() >= 500) && retries < max_retries {
            // Extract retry-after header if available
            let retry_after = if let Some(retry_header) = response.headers().get("retry-after") {
                if let Ok(retry_secs) = retry_header.to_str().unwrap_or("0").parse::<u64>() {
                    Some(Duration::from_secs(retry_secs))
                } else {
                    None
                }
            } else {
                None
            };

            // Use retry-after if available, otherwise use exponential backoff
            let wait_time = retry_after.unwrap_or(delay);

            retries += 1;
            debug_log(
                debug_level,
                DebugLevel::Minimal,
                &format!(
                    "API request failed with status {}, retrying in {} seconds (attempt {}/{})",
                    status, wait_time.as_secs(), retries, max_retries
                )
            );

            println!("{} Retrying in {} seconds (attempt {}/{})",
                "Rate limited by OpenAI API.".yellow().bold(),
                wait_time.as_secs(), retries, max_retries);

            tokio::time::sleep(wait_time).await;

            // Increase delay for next potential retry (exponential backoff)
            delay = std::cmp::min(
                Duration::from_secs((delay.as_secs() as f64 * backoff_factor) as u64),
                max_delay
            );

            continue;
        }

        // For non-retryable errors, just return the error
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API error: {} - {}", status, error_text));
        }

        // Success case
        let api_response: ApiResponse = response.json().await?;

        if debug_level >= DebugLevel::Minimal {
            debug_log(debug_level, DebugLevel::Minimal, "=== API RESPONSE ===");

            if let Some(tool_calls) = &api_response.choices[0].message.tool_calls {
                if debug_level >= DebugLevel::Verbose {
                    debug_log(
                        debug_level,
                        DebugLevel::Verbose,
                        &format!("Tool calls: {}", serde_json::to_string_pretty(tool_calls)?)
                    );
                } else {
                    debug_log(
                        debug_level,
                        DebugLevel::Minimal,
                        &format!("Tool calls: {} found", tool_calls.len())
                    );
                }
            } else {
                debug_log(debug_level, DebugLevel::Minimal, "No tool calls");
            }

            debug_log(debug_level, DebugLevel::Minimal, "=====================\n");
        }

        return Ok(api_response);
    }
}

pub async fn chat_with_ollama(
    client: &Client,
    _model_name: &str,
    messages: Vec<ResponseMessage>,
    debug_level: DebugLevel,
) -> Result<ApiResponse> {
    debug_log(debug_level, DebugLevel::Minimal, "\n=== SENDING MESSAGES TO OLLAMA MODEL ===");

    let url = format!("http://localhost:11434/v1/chat/completions");

    let request_body = json!({
        "messages": messages
    });

    if debug_level >= DebugLevel::Verbose {
        debug_log(
            debug_level,
            DebugLevel::Verbose,
            &format!("Request JSON: {}", serde_json::to_string_pretty(&request_body)?)
        );
    }

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();

    if !status.is_success() {
        let error_text = response.text().await?;
        return Err(anyhow!("Ollama API error: {} - {}", status, error_text));
    }

    let api_response: ApiResponse = response.json().await?;

    if debug_level >= DebugLevel::Minimal {
        debug_log(debug_level, DebugLevel::Minimal, "=== OLLAMA API RESPONSE ===");

        if let Some(tool_calls) = &api_response.choices[0].message.tool_calls {
            if debug_level >= DebugLevel::Verbose {
                debug_log(
                    debug_level,
                    DebugLevel::Verbose,
                    &format!("Tool calls: {}", serde_json::to_string_pretty(tool_calls)?)
                );
            } else {
                debug_log(
                    debug_level,
                    DebugLevel::Minimal,
                    &format!("Tool calls: {} found", tool_calls.len())
                );
            }
        } else {
            debug_log(debug_level, DebugLevel::Minimal, "No tool calls");
        }

        debug_log(debug_level, DebugLevel::Minimal, "=====================\n");
    }

    Ok(api_response)
}
