// volition-cli/src/tools/cargo.rs
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use tracing::warn;

// Import the core execution function and the output struct
use volition_agent_core::tools::cargo::execute_cargo_command as execute_cargo_command_core;
use volition_agent_core::tools::CommandOutput;

fn get_denied_cargo_commands() -> HashSet<String> {
    let mut denied = HashSet::new();
    denied.insert("login".to_string());
    denied.insert("logout".to_string());
    denied.insert("publish".to_string());
    denied.insert("owner".to_string());
    denied.insert("yank".to_string());
    denied.insert("install".to_string());
    denied
}

pub async fn run_cargo_command(
    command_name: &str,
    command_args: &[String],
    working_dir: &Path,
) -> Result<String> { // Returns String for ToolProvider
    let denied_commands = get_denied_cargo_commands();

    if denied_commands.contains(command_name) {
        warn!(
            "Denied execution of cargo command: cargo {} {:?}",
            command_name, command_args
        );
        return Ok(format!(
            "Error: The cargo command '{}' is not allowed for security reasons.",
            command_name
        ));
    }

    // Call the core library implementation
    let cmd_output: CommandOutput = 
        execute_cargo_command_core(command_name, command_args, working_dir).await?;

    // Format the structured output into a string for the AI/ToolProvider
    let command_str = format!("cargo {} {}", command_name, command_args.join(" "));
    Ok(cmd_output.format_for_ai(&command_str))
}
