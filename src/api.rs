use anyhow::{anyhow, Result};
use colored::*;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::Duration;

// Removed unused import: std::collections::HashMap
use crate::models::chat::{ApiResponse, ResponseMessage};
use log::{debug, info};
use crate::config::Config;

// Type alias for tool definition
pub type ToolDefinition = Value;

pub async fn chat_with_api(
    client: &Client,
    config: &Config,
    messages: Vec<ResponseMessage>,
    tools: Vec<ToolDefinition>,
    temperature: Option<f64>,
) -> Result<ApiResponse> {
    // Use provided temperature or default from config
    let effective_temperature = temperature.unwrap_or_else(|| config.default_temperature.unwrap_or(0.2));

    let (url, api_key) = match config.service.as_str() {
        "openai" => ("https://api.openai.com/v1/chat/completions", Some(&config.openai_api_key)),
        "ollama" => ("http://localhost:11434/v1/chat/completions", None),
        _ => return Err(anyhow!("Unsupported service: {}", config.service)),
    };

    let request_body = json!({
        "model": config.model_name,
        "messages": messages,
        "tools": tools, // Use provided tools
        "temperature": effective_temperature
    });

    debug!("Request JSON: {}", serde_json::to_string_pretty(&request_body)?);

    // Exponential backoff parameters
    let max_retries = 5;
    let initial_delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(60);
    let backoff_factor = 2.0;

    // Retry loop with exponential backoff
    let mut retries = 0;
    let mut delay = initial_delay;

    loop {
        let mut request = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request_body);

        if let Some(key) = api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await?;

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
            info!(
                "API request failed with status {}, retrying in {} seconds (attempt {}/{})",
                status, wait_time.as_secs(), retries, max_retries
            );

            println!("{} Retrying in {} seconds (attempt {}/{})",
                "Rate limited by API.".yellow().bold(),
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

        info!("=== API RESPONSE ===");

        if let Some(tool_calls) = &api_response.choices[0].message.tool_calls {
            debug!("Tool calls: {}", serde_json::to_string_pretty(tool_calls)?);
        } else {
            info!("No tool calls");
        }

        info!("=====================\n");

        return Ok(api_response);
    }
}