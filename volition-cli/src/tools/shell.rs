// volition-cli/src/tools/shell.rs
use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use std::path::Path;
use tracing::warn;

use volition_agent_core::tools::shell::execute_shell_command as execute_shell_command_core;
use volition_agent_core::tools::CommandOutput;

pub async fn run_shell_command(command: &str, working_dir: &Path) -> Result<String> {
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

    println!("{} {}", "Running:".blue().bold(), command);

    let cmd_output: CommandOutput = execute_shell_command_core(command, working_dir).await?;

    let shell_executable = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "sh"
    };
    let shell_arg = if cfg!(target_os = "windows") {
        "/C"
    } else {
        "-c"
    };
    let command_str_for_ai = format!("{} {} {}", shell_executable, shell_arg, command);
    Ok(format!(
        "Command executed: {}\nStatus: {}\nStdout:\n{}\nStderr:\n{}",
        command_str_for_ai,
        cmd_output.status,
        if cmd_output.stdout.is_empty() {
            "<no output>"
        } else {
            &cmd_output.stdout
        },
        if cmd_output.stderr.is_empty() {
            "<no output>"
        } else {
            &cmd_output.stderr
        }
    ))
}
