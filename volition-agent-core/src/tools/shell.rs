// volition-agent-core/src/tools/shell.rs

//! Core implementation for executing shell commands.

use super::CommandOutput;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{debug, warn};

// #[cfg(test)] // Removed mockall use
// use mockall::automock;

// #[cfg_attr(test, automock)] // Removed mockall attribute
/// Executes an arbitrary shell command in a specified working directory.
///
/// This function uses the platform's default shell (`sh -c` on Unix, `cmd /C` on Windows).
/// It captures stdout, stderr, and the exit status.
///
/// **Warning:** This function executes arbitrary commands as provided.
/// It does **not** perform any sandboxing, validation, or user confirmation.
/// Callers **must** ensure the command is safe to execute or implement appropriate
/// safety measures (like user confirmation) before calling this function.
/// Consider using more specific tool functions (e.g., `execute_git_command`)
/// where possible.
///
/// # Arguments
///
/// * `command`: The command string to execute via the shell.
/// * `working_dir`: The directory in which to execute the command.
///
/// # Returns
///
/// A `Result` containing a [`CommandOutput`] struct with the status, stdout, and stderr,
/// or an error if the process failed to spawn.
pub async fn execute_shell_command(command: &str, working_dir: &Path) -> Result<CommandOutput> {
    debug!("Executing shell command: {} in {:?}", command, working_dir);

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

    let output_result = Command::new(shell_executable)
        .current_dir(working_dir)
        .arg(shell_arg)
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("Failed to spawn shell process for command: {}", command));

    let output = match output_result {
        Ok(out) => out,
        Err(e) => {
            warn!(command = command, error = %e, "Failed to spawn command process");
            return Err(e);
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);

    debug!(
        "Shell command exit status: {}\nStdout preview (first 3 lines):\n{}\nStderr preview (first 3 lines):\n{}",
        status,
        stdout.lines().take(3).collect::<Vec<_>>().join("\n"),
        stderr.lines().take(3).collect::<Vec<_>>().join("\n")
    );

    Ok(CommandOutput {
        status,
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio;

    fn test_working_dir() -> PathBuf {
        tempdir().map(|d| d.into_path()).unwrap_or_default()
    }

    #[tokio::test]
    async fn test_execute_shell_echo() {
        let command = "echo Hello Core Shell";
        let working_dir = test_working_dir();
        let result = execute_shell_command(command, &working_dir).await;
        assert!(result.is_ok(), "Command failed: {:?}", result.err());
        let output = result.unwrap();
        println!("Output: {:?}", output);
        assert_eq!(output.status, 0);
        assert_eq!(output.stdout.trim(), "Hello Core Shell");
        assert!(output.stderr.is_empty() || output.stderr == "<no output>");
    }

    #[tokio::test]
    async fn test_execute_shell_nonexistent_command() {
        let command = "this_command_does_not_exist_qwertyuiop";
        let working_dir = test_working_dir();
        let result = execute_shell_command(command, &working_dir).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("Output: {:?}", output);
        assert_ne!(output.status, 0);
        assert!(output.stdout.is_empty() || output.stdout == "<no output>");
        assert!(output.stderr.contains("not found") || output.stderr.contains("is not recognized"));
    }
}
