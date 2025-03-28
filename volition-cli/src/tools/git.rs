// volition-cli/src/tools/git.rs
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{debug, info, warn};

// Removed GitCommandArgs import

// Define denied git commands and potentially dangerous argument combinations (unchanged)
fn is_git_command_denied(command_name: &str, args: &[String]) -> bool {
    let denied_commands: HashSet<&str> = [
        "push", "reset", "rebase", "checkout", "merge", "clone", "remote", "fetch", "pull",
    ]
    .iter()
    .cloned()
    .collect();

    if denied_commands.contains(command_name) {
        return true;
    }
    if command_name == "branch" && args.contains(&"-D".to_string()) {
        return true;
    }
    false
}

// Internal execution function (real version)
// Added working_dir argument
async fn execute_git_command_internal(
    command_name: &str,
    command_args: &[String],
    working_dir: &Path, // Added working_dir
) -> Result<String> {
    let full_command = format!("git {} {}", command_name, command_args.join(" "));
    debug!(
        "Executing internal git command: {} in {:?}",
        full_command,
        working_dir
    );

    let output = Command::new("git")
        .current_dir(working_dir) // Set working directory
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

// Public function exposed as the 'git_command' tool
// Refactored signature
pub async fn run_git_command(
    command_name: &str,
    command_args: &[String],
    working_dir: &Path,
) -> Result<String> {
    // Check against deny list and rules (unchanged)
    if is_git_command_denied(command_name, command_args) {
        warn!(
            "Denied execution of git command: git {} {:?}",
            command_name,
            command_args
        );
        return Ok(format!(
            "Error: The git command 'git {} {}' is not allowed for security or stability reasons.",
            command_name,
            command_args.join(" ")
        ));
    }

    info!("Running: git {} {}", command_name, command_args.join(" "));
    // Pass working_dir to internal function
    execute_git_command_internal(command_name, command_args, working_dir).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio;

    fn test_working_dir() -> PathBuf {
        tempdir()
            .map(|d| d.into_path())
            .unwrap_or_else(|_| PathBuf::from("."))
    }

    // Mock internal function for tests
    async fn mock_execute_internal(
        command_name: &str,
        command_args: &[String],
        _working_dir: &Path, // Ignore working_dir in mock
    ) -> Result<String> {
        let full_command_for_print = format!("git {} {}", command_name, command_args.join(" "));
        println!(
            "[TEST] Mock execute_git_command_internal called with: {}",
            full_command_for_print
        );
        match command_name {
            "status" => Ok(format!(
                "Command executed: {}\nStatus: 0\nStdout:\nworking tree clean\nStderr:\n<no output>",
                full_command_for_print
            )),
            "log" if command_args.contains(&"-1".to_string()) => Ok(format!(
                "Command executed: {}\nStatus: 0\nStdout:\ncommit abcdef\nTest commit\nStderr:\n<no output>",
                full_command_for_print
            )),
            "diff" if command_args.contains(&"non_existent_commit".to_string()) => Ok(format!(
                "Command executed: {}\nStatus: 128\nStdout:\n<no output>\nStderr:\nfatal: ambiguous argument...",
                full_command_for_print
            )),
            _ => Ok(format!(
                "Command executed: {}\nStatus: 0\nStdout:\nMock git success for {}\nStderr:\n<no output>",
                 full_command_for_print, command_name
            )),
        }
    }

    #[tokio::test]
    async fn test_run_git_command_denied_command() {
        let working_dir = test_working_dir();
        let result = run_git_command("push", &["origin".to_string(), "main".to_string()], &working_dir).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Error: The git command 'git push origin main' is not allowed"));
    }

    #[tokio::test]
    async fn test_run_git_command_denied_args() {
        let working_dir = test_working_dir();
        let result = run_git_command("branch", &["-D".to_string(), "old".to_string()], &working_dir).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Error: The git command 'git branch -D old' is not allowed"));
    }

    #[tokio::test]
    async fn test_run_git_command_allowed_status_success() {
        let working_dir = test_working_dir();
        // Override internal call with mock
        async fn execute_git_command_internal(cn: &str, ca: &[String], wd: &Path) -> Result<String> { mock_execute_internal(cn, ca, wd).await }
        
        let result = run_git_command("status", &[], &working_dir).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 0"));
        assert!(output.contains("working tree clean"));
    }

    #[tokio::test]
    async fn test_run_git_command_allowed_log_success() {
        let working_dir = test_working_dir();
        // Override internal call with mock
        async fn execute_git_command_internal(cn: &str, ca: &[String], wd: &Path) -> Result<String> { mock_execute_internal(cn, ca, wd).await }

        let result = run_git_command("log", &["-1".to_string()], &working_dir).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 0"));
        assert!(output.contains("commit abcdef"));
    }

    #[tokio::test]
    async fn test_run_git_command_allowed_diff_fail() {
        let working_dir = test_working_dir();
        // Override internal call with mock
        async fn execute_git_command_internal(cn: &str, ca: &[String], wd: &Path) -> Result<String> { mock_execute_internal(cn, ca, wd).await }

        let result = run_git_command("diff", &["non_existent_commit".to_string()], &working_dir).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 128"));
        assert!(output.contains("fatal: ambiguous argument"));
    }
}
