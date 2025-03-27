// src/tools/mod.rs
pub mod cargo;
pub mod file;
pub mod filesystem;
pub mod git;
pub mod search;
pub mod shell;
pub mod user_input;

use crate::config::RuntimeConfig;
use crate::models::chat::ResponseMessage;
use crate::models::tools::{
    CargoCommandArgs, FindRustDefinitionArgs, GitCommandArgs, ListDirectoryArgs, ReadFileArgs,
    SearchTextArgs, ShellArgs, ToolCall, UserInputArgs, WriteFileArgs,
};
// Removed unused `anyhow` macro import
use anyhow::{Context, Result};
use colored::*; // Import colored crate traits
use reqwest::Client;
use serde_json::from_str;
use tracing::{info, warn}; // Keeping info for internal logging

const MAX_PREVIEW_LINES: usize = 6; // Keep for preview in stdout

pub async fn handle_tool_calls(
    _client: &Client,
    config: &RuntimeConfig,
    tool_calls: Vec<ToolCall>,
    messages: &mut Vec<ResponseMessage>,
) -> Result<()> {
    info!("Processing {} tool calls", tool_calls.len()); // Internal log

    for tool_call in tool_calls.iter() {
        let tool_name = tool_call.function.name.as_str();
        let tool_args_json = &tool_call.function.arguments;

        // Print execution info to stdout for the user
        // NOTE: This println! makes testing output difficult. Consider refactoring later.
        println!(
            "\n{} {} ({})",
            "Running:".bold().cyan(),
            tool_name.bold(),
            tool_args_json.dimmed() // Display args dimmed
        );

        // Internal log (keeping this for debugging)
        info!(
            tool_name = tool_name,
            tool_args = tool_args_json,
            "Executing tool internally"
        );

        let output_result = match tool_name {
            "shell" => {
                let args: ShellArgs =
                    from_str(tool_args_json).context("Failed to parse shell arguments")?;
                shell::run_shell_command(args).await
            }
            "read_file" => {
                let args: ReadFileArgs =
                    from_str(tool_args_json).context("Failed to parse read_file arguments")?;
                file::read_file(args).await
            }
            "write_file" => {
                let args: WriteFileArgs =
                    from_str(tool_args_json).context("Failed to parse write_file arguments")?;
                file::write_file(args, config).await
            }
            "search_text" => {
                let args: SearchTextArgs =
                    from_str(tool_args_json).context("Failed to parse search_text arguments")?;
                search::search_text(args).await
            }
            "find_rust_definition" => {
                let args: FindRustDefinitionArgs = from_str(tool_args_json)
                    .context("Failed to parse find_rust_definition arguments")?;
                search::find_rust_definition(args).await
            }
            "user_input" => {
                let args: UserInputArgs =
                    from_str(tool_args_json).context("Failed to parse user_input arguments")?;
                user_input::get_user_input(args)
            }
            "cargo_command" => {
                let args: CargoCommandArgs =
                    from_str(tool_args_json).context("Failed to parse cargo_command arguments")?;
                cargo::run_cargo_command(args).await
            }
            "git_command" => {
                let args: GitCommandArgs =
                    from_str(tool_args_json).context("Failed to parse git_command arguments")?;
                git::run_git_command(args).await
            }
            "list_directory" => {
                let args: ListDirectoryArgs =
                    from_str(tool_args_json).context("Failed to parse list_directory arguments")?;
                filesystem::list_directory_contents(&args.path, args.depth, args.show_hidden)
            }
            unknown_tool => {
                warn!(tool_name = unknown_tool, "Attempted to call unknown tool");
                // Use anyhow::anyhow! fully qualified since we removed the direct import
                Err(anyhow::anyhow!("Unknown tool: {}", unknown_tool))
            }
        };

        match output_result {
            Ok(output) => {
                // Print success message to stdout for the user
                let preview: String = output
                    .lines()
                    .take(MAX_PREVIEW_LINES)
                    .collect::<Vec<&str>>()
                    .join("\n");
                let truncated = output.lines().count() > MAX_PREVIEW_LINES;
                let preview_suffix = if truncated { "... (truncated)" } else { "" };
                println!(
                    "{} {}\n{}{}",
                    "Result:".bold().green(),
                    tool_name.bold(),
                    preview.dimmed(), // Show preview dimmed
                    preview_suffix.dimmed()
                );

                // Internal log (keeping this for debugging)
                info!(
                    tool_name = tool_name,
                    output_preview = format!("{}{}", preview, preview_suffix).as_str(),
                    "Tool executed successfully internally"
                );

                // Push the *original* tool output to the messages for the AI
                messages.push(ResponseMessage {
                    role: "tool".to_string(),
                    content: Some(output), // Original output
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            }
            Err(e) => {
                // Print error message to stdout for the user
                println!(
                    "{} {} failed: {}",
                    "Error:".bold().red(),
                    tool_name.bold(),
                    e.to_string().red()
                );

                // Log the error internally (unchanged)
                warn!(
                    tool_name = tool_call.function.name.as_str(),
                    error = e.to_string().as_str(),
                    "Tool execution failed internally"
                );
                // Propagate the error up (unchanged)
                // Add context about which tool failed during execution
                return Err(e.context(format!(
                    "Failed during execution of tool: {}",
                    tool_call.function.name
                )));
            }
        }
    } // End loop through tool_calls

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuntimeConfig;
    // Allow unused import warning for ResponseMessage as it's needed for handle_tool_calls signature
    #[allow(unused_imports)]
    use crate::models::chat::ResponseMessage;
    use crate::models::tools::{ToolCall, ToolFunction};
    use reqwest::Client;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tokio;
    // Removed unused anyhow import from test scope

    // Helper to create a dummy RuntimeConfig for tests
    fn create_dummy_config() -> RuntimeConfig {
        RuntimeConfig {
            system_prompt: "".to_string(),
            selected_model: "".to_string(),
            models: HashMap::new(),
            api_key: "".to_string(),
            project_root: PathBuf::from("."),
        }
    }

    // Helper to create a dummy ToolCall for testing
    fn create_tool_call(id: &str, name: &str, args: &str) -> ToolCall {
        ToolCall {
            id: id.to_string(),
            call_type: "function".to_string(),
            function: ToolFunction {
                name: name.to_string(),
                arguments: args.to_string(),
            },
        }
    }

    #[tokio::test]
    async fn test_handle_tool_calls_shell_invalid_json() {
        let client = Client::new();
        let config = create_dummy_config();
        let mut messages = vec![];
        // Invalid JSON syntax in args
        let tool_calls = vec![create_tool_call("id1", "shell", r#"{"command: "echo"}"#)]; // Missing quote

        let result = handle_tool_calls(&client, &config, tool_calls, &mut messages).await;

        assert!(result.is_err());
        let error_string = format!("{:?}", result.err().unwrap());
        // Check the context message which is more stable
        assert!(error_string.contains("Failed to parse shell arguments"));
        assert_eq!(messages.len(), 0);
    }

     #[tokio::test]
    async fn test_handle_tool_calls_read_file_invalid_json() {
        let client = Client::new();
        let config = create_dummy_config();
        let mut messages = vec![];
        // Invalid JSON type
        let tool_calls = vec![create_tool_call("id2", "read_file", r#"{"path": 123}"#)]; // Path should be string

        let result = handle_tool_calls(&client, &config, tool_calls, &mut messages).await;

        assert!(result.is_err());
        let error_string = format!("{:?}", result.err().unwrap());
        // Check the context message
        assert!(error_string.contains("Failed to parse read_file arguments"));
        assert_eq!(messages.len(), 0);
    }


     #[tokio::test]
    async fn test_handle_tool_calls_shell_missing_required_arg() { // Renamed test
        let client = Client::new();
        let config = create_dummy_config();
        let mut messages = vec![];
         // Missing 'command' field
        let tool_calls = vec![create_tool_call("id1", "shell", r#"{"working_dir": "/tmp"}"#)];

        let result = handle_tool_calls(&client, &config, tool_calls, &mut messages).await;

        assert!(result.is_err());
        let error_string = format!("{:?}", result.err().unwrap());
        assert!(error_string.contains("Failed to parse shell arguments"));
        // Check for serde's missing field error text, which is reasonably stable
        assert!(error_string.contains("missing field `command`"));
        assert_eq!(messages.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_tool_calls_unknown_tool() {
        let client = Client::new();
        let config = create_dummy_config();
        let mut messages = vec![];
        let tool_calls = vec![
            create_tool_call("id1", "nonexistent_tool", r#"{}"#),
        ];

        let result = handle_tool_calls(&client, &config, tool_calls, &mut messages).await;

        assert!(result.is_err(), "Expected error due to unknown tool, but got Ok");
        let error_string = format!("{:?}", result.err().unwrap());
         // The error comes from the `handle_tool_calls` match statement directly
        assert!(error_string.contains("Unknown tool: nonexistent_tool"));
        assert_eq!(messages.len(), 0);
    }

    // TODO: Add tests for successful dispatch and output handling (requires mocking sub-functions)
}
