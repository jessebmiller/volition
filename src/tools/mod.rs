// src/tools/mod.rs
pub mod shell;
pub mod file;
pub mod search;
pub mod user_input;
pub mod cargo;
pub mod git;
pub mod filesystem; // Added

use anyhow::Result;
use reqwest::Client;
use crate::models::chat::ResponseMessage;
// Updated imports
use crate::models::tools::{
    CargoCommandArgs, GitCommandArgs, ListDirectoryArgs, ReadFileArgs, SearchTextArgs, ShellArgs, // Added ListDirectoryArgs
    UserInputArgs, WriteFileArgs, FindDefinitionArgs, ToolCall, // Added FindDefinitionArgs
};
use crate::config::RuntimeConfig;
use serde_json::from_str;
use tracing::{info};

pub async fn handle_tool_calls(
    _client: &Client,
    config: &RuntimeConfig,
    tool_calls: Vec<ToolCall>,
    messages: &mut Vec<ResponseMessage>
) -> Result<()> {
    info!("Processing {} tool calls", tool_calls.len());

    for (i, tool_call) in tool_calls.iter().enumerate() {
        info!("Processing tool call #{}: id={}, name={}", i, tool_call.id, tool_call.function.name);

        let output = match tool_call.function.name.as_str() {
             "shell" => {
                let args: ShellArgs = from_str(&tool_call.function.arguments)?;
                shell::run_shell_command(args).await?
            },
            "read_file" => {
                let args: ReadFileArgs = from_str(&tool_call.function.arguments)?;
                file::read_file(args).await?
            },
            "write_file" => {
                let args: WriteFileArgs = from_str(&tool_call.function.arguments)?;
                file::write_file(args, config).await?
            },
            "search_text" => {
                let args: SearchTextArgs = from_str(&tool_call.function.arguments)?;
                search::search_text(args).await?
            },
            "find_definition" => {
                let args: FindDefinitionArgs = from_str(&tool_call.function.arguments)?;
                search::find_definition(args).await?
            },
            "user_input" => {
                let args: UserInputArgs = from_str(&tool_call.function.arguments)?;
                user_input::get_user_input(args)?
            },
            "cargo_command" => {
                let args: CargoCommandArgs = from_str(&tool_call.function.arguments)?;
                cargo::run_cargo_command(args).await?
            },
            "git_command" => {
                let args: GitCommandArgs = from_str(&tool_call.function.arguments)?;
                git::run_git_command(args).await?
            },
            // Added list_directory handler
            "list_directory" => {
                let args: ListDirectoryArgs = from_str(&tool_call.function.arguments)?;
                // Call the function from the filesystem module (not async)
                filesystem::list_directory_contents(&args.path, args.depth, args.show_hidden)?
            },
            _ => {
                // Log unknown tool instead of returning Err immediately? Or keep as Err?
                // For now, keep as Err to make it obvious something is wrong.
                return Err(anyhow::anyhow!("Unknown tool: {}", tool_call.function.name));
            }
        };

        messages.push(ResponseMessage {
            role: "tool".to_string(),
            content: Some(output),
            tool_calls: None,
            tool_call_id: Some(tool_call.id.clone()),
        });
    }

    Ok(())
}
