// src/tools/git.rs
use crate::models::tools::GitCommandArgs;
use anyhow::Result; // Only Result needed by both
use std::collections::HashSet;
use tracing::{info, warn}; // info and warn needed by both

// Imports only needed for the non-test version
#[cfg(not(test))]
use {
    anyhow::Context, // Context only used in real execution
    std::process::{Command, Stdio},
    tracing::debug, // debug only used in real execution
};

// Define denied git commands and potentially dangerous argument combinations
fn is_git_command_denied(command_name: &str, args: &[String]) -> bool {
    let denied_commands: HashSet<&str> = [
        "push", "reset", "rebase", "checkout",
        "merge",  // Potentially destructive or state-changing
        "clone",  // Usually safe, but could be restricted if needed
        "remote", // Could be used to add malicious remotes
        "fetch",  // Could fetch from untrusted sources if combined with remote changes
        "pull",   // Combines fetch and merge/rebase
    ]
    .iter()
    .cloned()
    .collect();

    if denied_commands.contains(command_name) {
        return true;
    }

    // Check for specific dangerous argument combinations
    if command_name == "branch" && args.contains(&"-D".to_string()) {
        return true; // Deny forced deletion of branches
    }

    false
}

// Internal execution function (real version)
#[cfg(not(test))]
async fn execute_git_command_internal(
    command_name: &str,
    command_args: &[String],
) -> Result<String> {
    let full_command = format!("git {} {}", command_name, command_args.join(" "));
    debug!("Executing internal git command: {}", full_command);

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

    debug!(
        "git {} {} exit status: {}",
        command_name,
        command_args.join(" "),
        status
    );

    let result = format!(
        "Command executed: git {} {}\nStatus: {}\nStdout:\n{}\nStderr:\n{}",
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

// Internal execution function (test mock version)
#[cfg(test)]
async fn execute_git_command_internal(
    command_name: &str,
    command_args: &[String],
) -> Result<String> {
    let full_command_for_print = format!("git {} {}", command_name, command_args.join(" "));
    println!(
        "[TEST] Mock execute_git_command_internal called with: {}",
        full_command_for_print
    );

    // Mock based on command_name and potentially args
    match command_name {
        "status" => Ok(format!(
            "Command executed: {}\nStatus: 0\nStdout:\nOn branch main\nYour branch is up to date with 'origin/main'.\n\nnothing to commit, working tree clean\nStderr:\n<no output>",
            full_command_for_print
        )),
        "log" if command_args.contains(&"-1".to_string()) => Ok(format!(
            "Command executed: {}\nStatus: 0\nStdout:\ncommit abcdef12345 (HEAD -> main, origin/main)\nAuthor: Test User <test@example.com>\nDate:   Mon Jan 1 12:00:00 2024 +0000\n\n    Test commit message\nStderr:\n<no output>",
            full_command_for_print
        )),
        "diff" if command_args.contains(&"non_existent_commit".to_string()) => Ok(format!( // Simulate diff failure
            "Command executed: {}\nStatus: 128\nStdout:\n<no output>\nStderr:\nfatal: ambiguous argument 'non_existent_commit': unknown revision or path not in the working tree.\n",
            full_command_for_print
        )),
        _ => Ok(format!( // Default mock for other allowed commands
            "Command executed: {}\nStatus: 0\nStdout:\nMock git success for {}
Stderr:\n<no output>",
             full_command_for_print, command_name
        )),
    }
}

// Public function exposed as the 'git_command' tool
pub async fn run_git_command(args: GitCommandArgs) -> Result<String> {
    let command_name = &args.command;
    let command_args = &args.args;

    // Check against deny list and rules
    if is_git_command_denied(command_name, command_args) {
        warn!(
            "Denied execution of git command: git {} {:?}",
            command_name, command_args
        );
        return Ok(format!(
            "Error: The git command 'git {} {}' is not allowed for security or stability reasons.",
            command_name,
            command_args.join(" ")
        ));
    }

    // If allowed, call the appropriate internal execution function
    info!("Running: git {} {}", command_name, command_args.join(" "));
    execute_git_command_internal(command_name, command_args).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_run_git_command_denied_command() {
        let args = GitCommandArgs {
            command: "push".to_string(), // Denied command
            args: vec!["origin".to_string(), "main".to_string()],
        };
        let result = run_git_command(args).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Error: The git command 'git push origin main' is not allowed"));
    }

    #[tokio::test]
    async fn test_run_git_command_denied_args() {
        let args = GitCommandArgs {
            command: "branch".to_string(), // Allowed command, but denied args
            args: vec!["-D".to_string(), "old-feature".to_string()],
        };
        let result = run_git_command(args).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(
            output.contains("Error: The git command 'git branch -D old-feature' is not allowed")
        );
    }

    #[tokio::test]
    async fn test_run_git_command_allowed_status_success() {
        let args = GitCommandArgs {
            command: "status".to_string(),
            args: vec![],
        };
        let result = run_git_command(args).await; // Calls mock internal
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 0"));
        assert!(output.contains("working tree clean"));
        assert!(output.contains("Stderr:\n<no output>"));
    }

    #[tokio::test]
    async fn test_run_git_command_allowed_log_success() {
        let args = GitCommandArgs {
            command: "log".to_string(),
            args: vec!["-1".to_string()],
        };
        let result = run_git_command(args).await; // Calls mock internal
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 0"));
        assert!(output.contains("commit abcdef12345"));
        assert!(output.contains("Test commit message"));
        assert!(output.contains("Stderr:\n<no output>"));
    }

    #[tokio::test]
    async fn test_run_git_command_allowed_diff_fail() {
        let args = GitCommandArgs {
            command: "diff".to_string(),
            args: vec!["non_existent_commit".to_string()],
        };
        let result = run_git_command(args).await; // Calls mock internal
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 128"));
        assert!(output.contains("Stdout:\n<no output>"));
        assert!(output.contains("fatal: ambiguous argument"));
    }
}
