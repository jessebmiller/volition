mod shell;
mod file;
mod code_search;
mod user_input;

use anyhow::Result;
use reqwest::Client;
use crate::models::chat::ResponseMessage;
use crate::models::tools::ToolCall;
use serde_json::from_str;
use tracing::{info};

pub async fn handle_tool_calls(
    _client: &Client,
    _api_key: &str,
    tool_calls: Vec<ToolCall>,
    messages: &mut Vec<ResponseMessage>
) -> Result<()> {
    // Log the processing of tool calls
    info!("Processing {} tool calls", tool_calls.len());

    for (i, tool_call) in tool_calls.iter().enumerate() {
        info!("Processing tool call #{}: id={}, name={}", i, tool_call.id, tool_call.function.name);

        match tool_call.function.name.as_str() {
            "shell" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = shell::run_shell_command(args).await?;

                messages.push(ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            },
            "read_file" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = file::read_file(args).await?;

                messages.push(ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            },
            "write_file" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = file::write_file(args).await?;

                messages.push(ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            },
            "search_code" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = code_search::search_code(args).await?;

                messages.push(ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            },
            "find_definition" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = code_search::find_definition(args).await?;

                messages.push(ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            },
            "user_input" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = user_input::get_user_input(args)?;

                messages.push(ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            },
            _ => {
                return Err(anyhow::anyhow!("Unknown tool: {}", tool_call.function.name));
            }
        }
    }

    Ok(())
}
