use reqwest::Client;
use crate::api::chat_with_api;
use crate::models::chat::ResponseMessage;
use crate::utils::DebugLevel;
use crate::config::Config;
use crate::tools::handle_tool_calls;
use anyhow::Result;
use std::io::{self, Write};

/// Linear Strategy
/// This function represents a basic linear processing strategy.
/// It allows the LLM to use tools until it's done, based on the provided goal and system prompt.
///
/// # Arguments
/// * `client` - The HTTP client for making API requests.
/// * `config` - The configuration for API access.
/// * `tools` - A collection of tools that the strategy can use.
/// * `user_goal` - The goal for the solution.
/// * `system_prompt` - The system prompt to guide the LLM.
/// * `debug_level` - The level of debug information to log.
/// * `messages` - The current conversation state.
///
/// # Returns
/// * A result containing the updated conversation state.
pub async fn linear_strategy(
    client: &Client,
    config: &Config,
    tools: Vec<String>,
    user_goal: &str,
    system_prompt: &str,
    debug_level: DebugLevel,
    mut messages: Vec<ResponseMessage>,
) -> Result<Vec<ResponseMessage>> {
    let mut conversation_active = true;

    while conversation_active {
        let response = chat_with_api(client, config, messages.clone(), debug_level, None).await?;
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
            if debug_level >= DebugLevel::Minimal {
                println!("Processing {} tool calls", tool_calls.len());
            }

            handle_tool_calls(
                client,
                &config.openai_api_key,
                tool_calls.to_vec(),
                &mut messages,
                debug_level
            ).await?;
        } else {
            conversation_active = false;
        }
    }

    Ok(messages)
}
