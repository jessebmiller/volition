// src/tools/cargo.rs
use std::process::{Command, Stdio};
// Removed bail from import
use crate::models::tools::CargoCommandArgs;
use anyhow::{Context, Result};
use std::collections::HashSet;
use tracing::{debug, info, warn};

// Define denied cargo commands
fn get_denied_cargo_commands() -> HashSet<String> {
    let mut denied = HashSet::new();
    // Commands that might change credentials, publish crates, or install global binaries
    denied.insert("login".to_string());
    denied.insert("logout".to_string());
    denied.insert("publish".to_string());
    denied.insert("owner".to_string());
    denied.insert("yank".to_string());
    denied.insert("install".to_string()); // Can install globally
                                          // Add any other commands deemed too risky
    denied
}

// Public function exposed as the 'cargo_command' tool
pub async fn run_cargo_command(args: CargoCommandArgs) -> Result<String> {
    let command_name = &args.command;
    let command_args = &args.args;
    let denied_commands = get_denied_cargo_commands();

    // Check against deny list
    if denied_commands.contains(command_name) {
        warn!(
            "Denied execution of cargo command: cargo {} {:?}",
            command_name, command_args
        );
        // Return a clear error message to the AI/user
        return Ok(format!(
            "Error: The cargo command '{}' is not allowed for security reasons.",
            command_name
        ));
    }

    // Construct the full command string for logging
    let full_command = format!("cargo {} {}", command_name, command_args.join(" "));
    info!("Running: {}", full_command);

    // Execute the command
    let output = Command::new("cargo")
        .arg(command_name)
        .args(command_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context(format!("Failed to execute cargo command: {}", full_command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);

    debug!(
        "cargo {} {} exit status: {}",
        command_name,
        command_args.join(" "),
        status
    );

    // Format the result like the shell tool's internal executor
    let result = format!(
        "Command executed: cargo {} {}\nStatus: {}\nStdout:\n{}\nStderr:\n{}",
        command_name,
        command_args.join(" "),
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
