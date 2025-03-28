// volition-cli/src/tools/shell.rs
use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use std::path::Path;
use tracing::warn;

// Import the core execution function
use volition_agent_core::tools::shell::execute_shell_command as execute_shell_command_core;

/// Wrapper for the shell tool execution used by CliToolProvider.
/// Includes CLI-specific confirmation prompt.
pub async fn run_shell_command(command: &str, working_dir: &Path) -> Result<String> {
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
        // Return Ok with message
        return Ok(format!(
            "Shell command execution denied by user: {}",
            command
        ));
    }
    // --- End Confirmation ---

    println!("{} {}", "Running:".blue().bold(), command);
    // Call the core library implementation
    execute_shell_command_core(command, working_dir).await
}
