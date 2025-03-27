use crate::models::tools::ShellArgs;
use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use tracing::{debug, warn};

// Internal function to execute a shell command without confirmation
pub(crate) async fn execute_shell_command_internal(command: &str) -> Result<String> {
    debug!("Executing internal command: {}", command);

    // Regular command execution logic
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute Windows command")?
    } else {
        Command::new("sh")
            .args(["-c", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute shell command")?
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);

    // --- Logging moved inside ---
    let stdout_preview = stdout.lines().take(3).collect::<Vec<&str>>().join(
        "
",
    );
    let stderr_preview = if !stderr.is_empty() {
        format!(
            "
Stderr preview: {}",
            stderr.lines().take(3).collect::<Vec<&str>>().join(
                "
"
            )
        )
    } else {
        String::new()
    };

    debug!(
        "Internal command exit status: {}
Output preview:
{}{}",
        status,
        if stdout_preview.is_empty() {
            "<no output>"
        } else {
            &stdout_preview
        },
        stderr_preview
    );

    let detailed_info = format!(
        "Stdout length: {} bytes, Stderr length: {} bytes, Total lines: {}",
        stdout.len(),
        stderr.len(),
        stdout.lines().count() + stderr.lines().count()
    );
    debug!("{}", detailed_info);
    // --- End Logging ---

    let result = format!(
        "Command executed with status: {}
Stdout:
{}
Stderr:
{}",
        status,
        if stdout.is_empty() {
            "<no output>"
        } else {
            &stdout
        },
        if stderr.is_empty() {
            "<no output>"
        } else {
            &stderr
        }
    );

    Ok(result)
}

// Public function exposed as the 'shell' tool, includes confirmation
pub async fn run_shell_command(args: ShellArgs) -> Result<String> {
    let command = &args.command;

    // --- Mandatory Confirmation (y/N style, default No) ---
    print!(
        "{}
{}
{}{} ",
        "WARNING: This tool can execute arbitrary code!"
            .red()
            .bold(),
        format!("Request to run shell command: {}", command).yellow(),
        "Allow execution? ".yellow(),
        "(y/N):".yellow().bold() // Default to No
    );
    // Ensure the prompt is displayed before reading input
    io::stdout().flush().context("Failed to flush stdout")?;

    let mut user_choice = String::new();
    io::stdin()
        .read_line(&mut user_choice)
        .context("Failed to read user input")?;

    // Only proceed if the user explicitly types 'y' (case-insensitive)
    if user_choice.trim().to_lowercase() != "y" {
        warn!("User denied execution of shell command: {}", command);
        println!("{}", "Shell command execution denied.".red());
        // Return a message indicating the command was skipped
        return Ok(format!(
            "Shell command execution denied by user: {}",
            command
        ));
    }
    // --- End Confirmation ---

    // If approved, call the internal execution function
    println!("{} {}", "Running:".blue().bold(), command);
    execute_shell_command_internal(command).await
}
