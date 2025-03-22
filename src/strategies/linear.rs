use reqwest::Client;
use crate::api::chat_with_api;
use crate::models::chat::ResponseMessage;
use log::info;
use crate::config::Config;
use crate::tools::handle_tool_calls;
use serde_json::Value; // Use Value from serde_json

/// Linear Strategy
/// This function represents a basic linear processing strategy.  It
/// allows the LLM to use tools until it's done, based on the current
/// conversation state.
///
/// # Arguments
/// * `client` - The HTTP client for making API requests.
/// * `config` - The configuration for API access.
/// * `tools` - A collection of tools that the strategy can use.
/// * `end_tool` - The tool that, when called, ends the strategy.
/// * `messages` - The current conversation state.
///
/// # Returns
/// * A result containing the updated conversation state.
pub async fn linear_strategy(
    client: &Client,
    config: &Config,
    tools: Vec<Value>,
    end_tool: &str,
    mut messages: Vec<ResponseMessage>,
) -> Result<Vec<ResponseMessage>, anyhow::Error> {
    let mut conversation_active = true;

    while conversation_active {
        let response = chat_with_api(client, config, messages.clone(), tools.clone(), config.default_temperature).await?;
        let message = &response.choices[0].message;

        if let Some(content) = &message.content {
            if !content.is_empty() {
                println!("\n{}", content);
            }
        }

        messages.push(ResponseMessage {
            role: "assistant".to_string(),
            content: message.content.clone(),
            tool_calls: message.tool_calls.clone(),
            tool_call_id: None,
        });

        if let Some(tool_calls) = &message.tool_calls {
            info!("Processing {} tool calls", tool_calls.len());

            handle_tool_calls(
                client,
                &config.openai_api_key,
                tool_calls.to_vec(),
                &mut messages,
            ).await?;

            // Check if the end tool was called
            if tool_calls.iter().any(|call| call == end_tool) {
                conversation_active = false;
            }
        } else {
            conversation_active = false;
        }
    }

    Ok(messages)
}
