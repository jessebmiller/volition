mod shell;
mod file;
mod code_search;
mod user_input;
mod submit_quality_score;

use anyhow::Result;
use reqwest::Client;
use crate::models::chat::ResponseMessage;
use crate::models::tools::ToolCall;

use serde_json::from_str;

pub async fn handle_tool_calls(
    _client: &Client,
    _api_key: &str,
    tool_calls: Vec<ToolCall>,
    messages: &mut Vec<ResponseMessage>,
) -> Result<()> {
    for (_i, tool_call) in tool_calls.iter().enumerate() {
        match tool_call.function.name.as_str() {
            "shell" => {
                let args = from_str(&tool_call.function.arguments)?;
                let _output = shell::run_shell_command(args).await?;

                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(_output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };

                messages.push(tool_message);
            },
            "read_file" => {
                let args = from_str(&tool_call.function.arguments)?;
                let _output = file::read_file(args).await?;

                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(_output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };

                messages.push(tool_message);
            },
            "write_file" => {
                let args = from_str(&tool_call.function.arguments)?;
                let _output = file::write_file(args).await?;

                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(_output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };

                messages.push(tool_message);
            },
            "search_code" => {
                let args = from_str(&tool_call.function.arguments)?;
                let _output = code_search::search_code(args).await?;

                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(_output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };

                messages.push(tool_message);
            },
            "find_definition" => {
                let args = from_str(&tool_call.function.arguments)?;
                let _output = code_search::find_definition(args).await?;

                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(_output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };

                messages.push(tool_message);
            },
            "user_input" => {
                let args = from_str(&tool_call.function.arguments)?;
                let _output = user_input::get_user_input(args)?;

                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(_output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                };

                messages.push(tool_message);
            },
            "submit_quality_score" => {
                let args = from_str(&tool_call.function.arguments)?;
                let _output = submit_quality_score::submit_quality_score(args).await?;

                let tool_message = ResponseMessage {
                    role: "tool".to_string(),
                    content: Some("Quality score submitted successfully.".to_string()),
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