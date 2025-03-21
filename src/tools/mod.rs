mod shell;
mod file;
mod code_search;
mod user_input;

use anyhow::Result;
use reqwest::Client;
use crate::models::chat::ResponseMessage;
use crate::models::tools::ToolCall;
use crate::utils::DebugLevel;
use crate::utils::debug_log;

pub use shell::run_shell_command;
pub use file::{read_file, write_file};
pub use code_search::{search_code, find_definition};
pub use user_input::get_user_input;

use serde_json::from_str;
use std::io::{self, Write};
use colored::*;

pub async fn handle_tool_calls(
    _client: &Client,
    _api_key: &str,
    tool_calls: Vec<ToolCall>,
    messages: &mut Vec<ResponseMessage>,
    debug_level: DebugLevel,
) -> Result<()> {
    for (i, tool_call) in tool_calls.iter().enumerate() {
        if debug_level >= DebugLevel::Minimal {
            debug_log(
                debug_level,
                DebugLevel::Minimal,
                &format!(
                    "Processing tool call #{}: id={}, name={}",
                    i, tool_call.id, tool_call.function.name
                )
            );
        }

        match tool_call.function.name.as_str() {
            "shell" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = shell::run_shell_command(args, debug_level).await?;
                
                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };
                
                messages.push(tool_message);
            },
            "read_file" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = file::read_file(args, debug_level).await?;
                
                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };
                
                messages.push(tool_message);
            },
            "write_file" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = file::write_file(args, debug_level).await?;
                
                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };
                
                messages.push(tool_message);
            },
            "search_code" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = code_search::search_code(args, debug_level).await?;
                
                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };
                
                messages.push(tool_message);
            },
            "find_definition" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = code_search::find_definition(args, debug_level).await?;
                
                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };
                
                messages.push(tool_message);
            },
            "user_input" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = user_input::get_user_input(args)?;
                
                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };
                
                messages.push(tool_message);
            },
            _ => {
                return Err(anyhow::anyhow!("Unknown tool: {}", tool_call.function.name));
            }
        }
    }

    Ok(())
}