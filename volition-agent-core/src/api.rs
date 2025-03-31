// volition-agent-core/src/api.rs

//! Handles interactions with external AI model APIs.

use crate::config::ModelConfig;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::models::tools::ToolDefinition;
use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Method};
// Removed unused Serialize import
use serde_json::json;
use tracing::{error, trace}; // Removed unused debug

/// Generic function to make a request to an AI chat completion API.
pub async fn get_chat_completion_generic(
    http_client: &Client,
    model_config: &ModelConfig,
    api_key: &str,
    messages: Vec<ChatMessage>,
    tools: Option<&[ToolDefinition]>,
) -> Result<ApiResponse> {
    let mut payload = json!({
        "model": model_config.model_name, // Use model_name from ModelConfig
        "messages": messages,
    });

    if let Some(tools) = tools {
        if !tools.is_empty() {
            payload["tools"] = json!(tools);
        }
    }

    // Handle Option<toml::Value> for parameters
    if let Some(params_value) = &model_config.parameters {
        if let Some(params_table) = params_value.as_table() {
            if let Some(payload_obj) = payload.as_object_mut() {
                for (key, value) in params_table {
                    let json_value: serde_json::Value = value.clone().try_into()
                        .context("Failed to convert TOML params to JSON")?;
                    payload_obj.insert(key.clone(), json_value);
                }
            }
        } else {
            // Handle case where parameters is a string or other non-table type if needed
            trace!("Model parameters are not a table, skipping merge.");
        }
    }

    let endpoint = model_config.endpoint.as_deref()
        .ok_or_else(|| anyhow!("Endpoint missing for model {}", model_config.model_name))?;

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
        return Err(anyhow!(
            "API request failed with status {}: {}\nCheck API key, endpoint, model name, and request payload.",
            status,
            response_text
        ));
    }

    serde_json::from_str::<ApiResponse>(&response_text)
        .with_context(|| format!("Failed to parse API response JSON: {}", response_text))
}

/* Old get_chat_completion commented out */
