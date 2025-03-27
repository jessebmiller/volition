// src/tools/mod.rs
pub mod cargo;
pub mod file;
pub mod filesystem;
pub mod git;
pub mod search;
pub mod shell;
pub mod user_input;

use crate::models::chat::ResponseMessage;
use anyhow::{Context, Result};
use reqwest::Client;
use crate::config::RuntimeConfig;
use crate::models::tools::{
    CargoCommandArgs,
    FindRustDefinitionArgs, // Updated import
    GitCommandArgs,
    ListDirectoryArgs,
    ReadFileArgs,
    SearchTextArgs,
    ShellArgs,
    ToolCall,
    UserInputArgs,
    WriteFileArgs,
};
use serde_json::from_str;
use tracing::{info, warn};

const MAX_PREVIEW_LINES: usize = 6;

pub async fn handle_tool_calls(
    _client: &Client,
    config: &RuntimeConfig,
    tool_calls: Vec<ToolCall>,
    messages: &mut Vec<ResponseMessage>,
) -> Result<()> {
    info!("Processing {} tool calls", tool_calls.len());

    for tool_call in tool_calls.iter() {
        info!(
            tool_name = tool_call.function.name.as_str(),
            tool_args = tool_call.function.arguments.as_str(),
            "Executing tool"
        );

        let output_result = match tool_call.function.name.as_str() {
            "shell" => {
                let args: ShellArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse shell arguments")?;
                shell::run_shell_command(args).await
            }
            "read_file" => {
                let args: ReadFileArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse read_file arguments")?;
                file::read_file(args).await
            }
            "write_file" => {
                let args: WriteFileArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse write_file arguments")?;
                file::write_file(args, config).await
            }
            "search_text" => {
                let args: SearchTextArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse search_text arguments")?;
                search::search_text(args).await
            }
            // Renamed tool handler
            "find_rust_definition" => {
                let args: FindRustDefinitionArgs = from_str(&tool_call.function.arguments) // Updated args type
                    .context("Failed to parse find_rust_definition arguments")?;
                search::find_rust_definition(args).await // Updated function call
            }
            "user_input" => {
                let args: UserInputArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse user_input arguments")?;
                user_input::get_user_input(args)
            }
            "cargo_command" => {
                let args: CargoCommandArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse cargo_command arguments")?;
                cargo::run_cargo_command(args).await
            }
            "git_command" => {
                let args: GitCommandArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse git_command arguments")?;
                git::run_git_command(args).await
            }
            "list_directory" => {
                let args: ListDirectoryArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse list_directory arguments")?;
                filesystem::list_directory_contents(&args.path, args.depth, args.show_hidden)
            }
            unknown_tool => {
                warn!(tool_name = unknown_tool, "Attempted to call unknown tool");
                Err(anyhow::anyhow!("Unknown tool: {}", unknown_tool))
            }
        };

        match output_result {
            Ok(output) => {
                let preview: String = output
                    .lines()
                    .take(MAX_PREVIEW_LINES)
                    .collect::<Vec<&str>>()
                    .join("\n");

                let truncated = output.lines().count() > MAX_PREVIEW_LINES;
                let preview_suffix = if truncated { "... (truncated)" } else { "" };

                info!(
                    tool_name = tool_call.function.name.as_str(),
                    output_preview = format!("{}{}", preview, preview_suffix).as_str(),
                    "Tool executed successfully"
                );

                messages.push(ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            }
            Err(e) => {
                warn!(
                    tool_name = tool_call.function.name.as_str(),
                    error = e.to_string().as_str(),
                    "Tool execution failed"
                );
                return Err(e.context(format!(
                    "Failed to execute tool: {}",
                    tool_call.function.name
                )));
            }
        }
    } // End loop through tool_calls

    Ok(())
}
