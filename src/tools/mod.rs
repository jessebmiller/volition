mod shell;
mod file;
mod search; // Renamed from code_search
mod user_input;

use anyhow::Result;
use reqwest::Client;
use crate::models::chat::ResponseMessage;
use crate::models::tools::ToolCall;
// Import RuntimeConfig
use crate::config::RuntimeConfig;
use serde_json::from_str;
use tracing::{info};

pub async fn handle_tool_calls(
    _client: &Client, // Marked as unused for now
    config: &RuntimeConfig, // Added config
    tool_calls: Vec<ToolCall>,
    messages: &mut Vec<ResponseMessage>
) -> Result<()> {
    // Log the processing of tool calls
    info!("Processing {} tool calls", tool_calls.len());

    for (i, tool_call) in tool_calls.iter().enumerate() {
        info!("Processing tool call #{}: id={}, name={}", i, tool_call.id, tool_call.function.name);

        // Use a temporary variable for output to simplify pushing to messages
        // Pass config where needed
        let output = match tool_call.function.name.as_str() {
            "shell" => {
                let args = from_str(&tool_call.function.arguments)?;
                // Don't pass config to shell command runner anymore
                shell::run_shell_command(args).await?
            },
            "read_file" => {
                let args = from_str(&tool_call.function.arguments)?;
                // Assuming read_file doesn't need config for now
                file::read_file(args).await?
            },
            "write_file" => {
                let args = from_str(&tool_call.function.arguments)?;
                // Pass config to write_file runner
                file::write_file(args, config).await?
            },
            // Updated to use search_text
            "search_text" => {
                let args = from_str(&tool_call.function.arguments)?;
                // Don't pass config to search_text
                search::search_text(args).await?
            },
            "find_definition" => {
                let args = from_str(&tool_call.function.arguments)?;
                // Don't pass config to find_definition
                search::find_definition(args).await?
            },
            "user_input" => {
                let args = from_str(&tool_call.function.arguments)?;
                user_input::get_user_input(args)?
            },
            _ => {
                return Err(anyhow::anyhow!("Unknown tool: {}", tool_call.function.name));
            }
        };

        // Push the result after the match statement
        messages.push(ResponseMessage {
            role: "tool".to_string(),
            content: Some(output),
            tool_calls: None,
            tool_call_id: Some(tool_call.id.clone()),
        });
    }

    Ok(())
}
