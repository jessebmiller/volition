// src/tools/git.rs
use std::process::{Command, Stdio};
// Removed bail from import
use anyhow::{Context, Result};
use tracing::{debug, warn, info};
use crate::models::tools::GitCommandArgs;
use std::collections::HashSet;

// Define denied git commands and potentially dangerous argument combinations
fn is_git_command_denied(command_name: &str, args: &[String]) -> bool {
    let denied_commands: HashSet<&str> = [
        "push", "reset", "rebase", "checkout", "merge", // Potentially destructive or state-changing
        "clone", // Usually safe, but could be restricted if needed
        "remote", // Could be used to add malicious remotes
        "fetch", // Could fetch from untrusted sources if combined with remote changes
        "pull", // Combines fetch and merge/rebase
        // Add other potentially risky commands
    ].iter().cloned().collect();

    if denied_commands.contains(command_name) {
        return true;
    }

    // Check for specific dangerous argument combinations
    if command_name == "branch" && args.contains(&"-D".to_string()) {
        return true; // Deny forced deletion of branches
    }
    // Add more specific checks if needed (e.g., git commit --amend without checking?)

    false
}

// Public function exposed as the 'git_command' tool
pub async fn run_git_command(args: GitCommandArgs) -> Result<String> {
    let command_name = &args.command;
    let command_args = &args.args;

    // Check against deny list and rules
    if is_git_command_denied(command_name, command_args) {
        warn!("Denied execution of git command: git {} {:?}", command_name, command_args);
        // Return a clear error message to the AI/user
        return Ok(format!("Error: The git command 'git {} {}' is not allowed for security or stability reasons.", command_name, command_args.join(" ")));
    }

    // Construct the full command string for logging
    let full_command = format!("git {} {}", command_name, command_args.join(" "));
    info!("Running: {}", full_command);

    // Execute the command
    let output = Command::new("git")
        .arg(command_name)
        .args(command_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context(format!("Failed to execute git command: {}", full_command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);

    debug!("git {} {} exit status: {}", command_name, command_args.join(" "), status);

    // Format the result like the shell tool's internal executor
    let result = format!(
        "Command executed: git {} {}\nStatus: {}\nStdout:\n{}\nStderr:\n{}",
        command_name,
        command_args.join(" "),
        status,
        if stdout.is_empty() { "<no output>" } else { &stdout },
        if stderr.is_empty() { "<no output>" } else { &stderr }
    );

    Ok(result)
}
