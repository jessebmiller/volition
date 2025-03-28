// volition-cli/src/tools/shell.rs
use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use std::path::Path;
use tracing::warn;

// Import the core execution function and the output struct
use volition_agent_core::tools::shell::execute_shell_command as execute_shell_command_core;
use volition_agent_core::tools::CommandOutput;

pub async fn run_shell_command(command: &str, working_dir: &Path) -> Result<String> { // Returns String
    // --- Mandatory Confirmation ---
    print!(
        "{}\n{}\n{}{} ",
        "WARNING: This tool can execute arbitrary code!"
            .red()
            .bold(),
        format!("Request to run shell command: {}", command).yellow(),
        "Allow execution? ".yellow(),
        "(y/N):".yellow().bold()
    );
    io::stdout().flush().context("Failed to flush stdout")?;

    let mut user_choice = String::new();
    io::stdin()
        .read_line(&mut user_choice)
        .context("Failed to read user input")?;

    if user_choice.trim().to_lowercase() != "y" {
        warn!("User denied execution of shell command: {}", command);
        println!("{}", "Shell command execution denied.".red());
        return Ok(format!(
            "Shell command execution denied by user: {}",
            command
        ));
    }
    // --- End Confirmation ---

    println!("{} {}", "Running:".blue().bold(), command);
    
    // Call the core library implementation
    let cmd_output: CommandOutput = execute_shell_command_core(command, working_dir).await?;

    // Format the structured output into a string
    // Determine the shell executable string used by the core function for formatting
    let shell_executable = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let shell_arg = if cfg!(target_os = "windows") { "/C" } else { "-c" };
    let command_str_for_ai = format!("{} {} {}", shell_executable, shell_arg, command);
    Ok(cmd_output.format_for_ai(&command_str_for_ai))
}
