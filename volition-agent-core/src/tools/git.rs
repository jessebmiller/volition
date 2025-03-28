// volition-agent-core/src/tools/git.rs

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{debug, info}; // Removed warn as deny list is removed

/// Executes a git command in a specified working directory.
///
/// Note: This does not perform safety checks (like denying push/reset).
/// Callers should ensure the command/args are safe or implement checks separately.
pub async fn execute_git_command(
    command_name: &str,
    command_args: &[String],
    working_dir: &Path,
) -> Result<String> {
    let full_command_log = format!("git {} {}", command_name, command_args.join(" "));
    info!(
        "Executing git command: {} in {:?}",
        full_command_log,
        working_dir
    );

    let output = Command::new("git")
        .current_dir(working_dir)
        .arg(command_name)
        .args(command_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("Failed to execute git command: {}", full_command_log))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);

    debug!(
        "git {} exit status: {}",
        full_command_log,
        status
    );

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::tempdir;
    use tokio;

    // Helper to initialize a dummy git repo in a temp dir
    fn setup_git_repo() -> Result<PathBuf> {
        let dir = tempdir()?.into_path();
        Command::new("git")
            .current_dir(&dir)
            .arg("init")
            .output()?;
        Command::new("git")
            .current_dir(&dir)
            .args(&["config", "user.email", "test@example.com"])
            .output()?;
        Command::new("git")
            .current_dir(&dir)
            .args(&["config", "user.name", "Test User"])
            .output()?;
        fs::write(dir.join("README.md"), "Initial commit")?;
        Command::new("git")
            .current_dir(&dir)
            .arg("add")
            .arg("README.md")
            .output()?;
        Command::new("git")
            .current_dir(&dir)
            .arg("commit")
            .arg("-m")
            .arg("Initial commit")
            .output()?;
        Ok(dir)
    }

    #[tokio::test]
    async fn test_execute_git_status_clean() {
        let working_dir = setup_git_repo().expect("Failed to setup git repo");
        let result = execute_git_command("status", &[], &working_dir).await;
        assert!(result.is_ok(), "git status failed: {:?}", result.err());
        let output = result.unwrap();
        println!("Output:\n{}", output);
        assert!(output.contains("Status: 0"));
        assert!(output.contains("nothing to commit, working tree clean"));
    }

    #[tokio::test]
    async fn test_execute_git_log_initial() {
        let working_dir = setup_git_repo().expect("Failed to setup git repo");
        let result = execute_git_command("log", &["-1".to_string()], &working_dir).await;
        assert!(result.is_ok(), "git log failed: {:?}", result.err());
        let output = result.unwrap();
        println!("Output:\n{}", output);
        assert!(output.contains("Status: 0"));
        assert!(output.contains("Initial commit"));
    }

    #[tokio::test]
    async fn test_execute_git_diff_fail() {
        let working_dir = setup_git_repo().expect("Failed to setup git repo");
        let result = execute_git_command("diff", &["nonexistentcommit".to_string()], &working_dir).await;
         assert!(result.is_ok(), "Expected Ok result even on diff failure");
        let output = result.unwrap();
        println!("Output:\n{}", output);
        assert!(output.contains("Status: 128")); // Git diff often exits with 128 on error
        assert!(output.contains("fatal: ambiguous argument"));
    }
}
