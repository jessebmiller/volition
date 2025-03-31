// volition-agent-core/src/api.rs

//! Handles interactions with external AI model APIs.

use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::models::tools::ToolDefinition;
use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Method};
use serde_json::json;
use tracing::{error, trace};

/// Generic function to make a request to an AI chat completion API.
///
/// This function constructs the request payload based on common patterns
/// (like OpenAI's API) and handles sending the request and parsing the response.
///
/// # Arguments
///
/// * `http_client`: The reqwest HTTP client.
/// * `endpoint`: The specific API endpoint URL for the provider.
/// * `api_key`: The API key for authentication.
/// * `model_name`: The specific model name to use.
/// * `messages`: The conversation history.
/// * `tools`: Optional list of tool definitions for the AI to use.
/// * `parameters`: Optional model parameters (like temperature) as a TOML Value.
///
/// # Errors
///
/// Returns an error if the request fails, the response cannot be parsed,
/// or the API returns an error status.
pub async fn call_chat_completion_api(
    http_client: &Client,
    endpoint: &str,
    api_key: &str,
    model_name: &str,
    messages: Vec<ChatMessage>,
    tools: Option<&[ToolDefinition]>,
    parameters: Option<&toml::Value>,
) -> Result<ApiResponse> {
    // Construct the base payload
    let mut payload = json!({
        "model": model_name,
        "messages": messages,
    });

    // Add tools if provided
    if let Some(tools) = tools {
        if !tools.is_empty() {
            payload["tools"] = json!(tools);
            // Consider adding tool_choice = "auto" or specific choice if needed
            // payload["tool_choice"] = json!("auto");
        }
    }

    // Merge model-specific parameters
    if let Some(params_value) = parameters {
        if let Some(params_table) = params_value.as_table() {
            if let Some(payload_obj) = payload.as_object_mut() {
                for (key, value) in params_table {
                    // Convert toml::Value to serde_json::Value
                    let json_value: serde_json::Value = value.clone().try_into()
                        .context("Failed to convert TOML params to JSON")?;
                    payload_obj.insert(key.clone(), json_value);
                }
            }
        } else {
            trace!("Model parameters are not a table, skipping merge.");
        }
    }

    trace!(endpoint = %endpoint, payload = %serde_json::to_string_pretty(&payload).unwrap_or_default(), "Sending API request");

    let response = http_client
        .request(Method::POST, endpoint)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .context("Failed to send request to AI model API")?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .context("Failed to read API response text")?;

    trace!(status = %status, response_body = %response_text, "Received API response");

    if !status.is_success() {
        error!(status = %status, response_body = %response_text, "API request failed");
        // Consider parsing error response body for more details
        return Err(anyhow!(
            "API request failed with status {}: {}\nCheck API key, endpoint, model name, and request payload.",
            status,
            response_text
        ));
    }

    serde_json::from_str::<ApiResponse>(&response_text)
        .with_context(|| format!("Failed to parse API response JSON: {}", response_text))
}

// Removed old get_chat_completion and get_chat_completion_generic
