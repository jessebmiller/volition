// src/tools/mod.rs
pub mod shell;
pub mod file;
pub mod search;
pub mod user_input;
pub mod cargo;
pub mod git;
pub mod filesystem; // Added

use anyhow::{Context, Result}; // Added Context
use reqwest::Client;
use crate::models::chat::ResponseMessage;
// Updated imports
use crate::models::tools::{
    CargoCommandArgs, GitCommandArgs, ListDirectoryArgs, ReadFileArgs, SearchTextArgs, ShellArgs, // Added ListDirectoryArgs
    UserInputArgs, WriteFileArgs, FindDefinitionArgs, ToolCall, // Added FindDefinitionArgs
};
use crate::config::RuntimeConfig;
use serde_json::from_str;
use tracing::{info, warn}; // Make sure warn is imported

// Define the number of preview lines
const MAX_PREVIEW_LINES: usize = 6;

pub async fn handle_tool_calls(
    _client: &Client,
    config: &RuntimeConfig,
    tool_calls: Vec<ToolCall>,
    messages: &mut Vec<ResponseMessage>
) -> Result<()> {
    info!("Processing {} tool calls", tool_calls.len());

    for tool_call in tool_calls.iter() { // No need for index `i` here anymore

        // Log the tool call details *before* execution
        info!(
            tool_name = tool_call.function.name.as_str(),
            tool_args = tool_call.function.arguments.as_str(),
            "Executing tool"
        );

        // Execute the tool call
        // Use `.context()` here for better error messages if argument parsing fails
        let output_result = match tool_call.function.name.as_str() {
             "shell" => {
                let args: ShellArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse shell arguments")?;
                shell::run_shell_command(args).await
            },
            "read_file" => {
                let args: ReadFileArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse read_file arguments")?;
                file::read_file(args).await
            },
            "write_file" => {
                let args: WriteFileArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse write_file arguments")?;
                file::write_file(args, config).await
            },
            "search_text" => {
                let args: SearchTextArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse search_text arguments")?;
                search::search_text(args).await
            },
            "find_definition" => {
                let args: FindDefinitionArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse find_definition arguments")?;
                search::find_definition(args).await
            },
            "user_input" => {
                let args: UserInputArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse user_input arguments")?;
                // user_input::get_user_input is sync and returns Result<String>
                user_input::get_user_input(args)
            },
            "cargo_command" => {
                let args: CargoCommandArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse cargo_command arguments")?;
                cargo::run_cargo_command(args).await
            },
            "git_command" => {
                let args: GitCommandArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse git_command arguments")?;
                git::run_git_command(args).await
            },
            "list_directory" => {
                let args: ListDirectoryArgs = from_str(&tool_call.function.arguments)
                    .context("Failed to parse list_directory arguments")?;
                // list_directory_contents is sync and returns Result<String>
                filesystem::list_directory_contents(&args.path, args.depth, args.show_hidden)
            },
            unknown_tool => {
                // Log and return error for unknown tool
                warn!(tool_name = unknown_tool, "Attempted to call unknown tool");
                // Use anyhow! macro for direct error creation
                Err(anyhow::anyhow!("Unknown tool: {}", unknown_tool))
            }
        };

        // Handle the result (Ok or Err)
        match output_result {
            Ok(output) => {
                // Log preview of the output
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
                    // Optionally log full output length for context
                    // output_len = output.len(),
                    "Tool executed successfully"
                );

                // Push the full result message
                messages.push(ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            }
            Err(e) => {
                 // Log the error
                 warn!(
                     tool_name = tool_call.function.name.as_str(),
                     error = e.to_string().as_str(),
                     "Tool execution failed"
                 );
                 // Propagate the error up, adding context about which tool failed.
                 return Err(e.context(format!("Failed to execute tool: {}", tool_call.function.name)));
            }
        }
    } // End loop through tool_calls

    Ok(())
}
