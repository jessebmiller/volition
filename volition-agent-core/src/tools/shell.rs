// volition-agent-core/src/tools/shell.rs

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{debug, warn}; // Use warn for spawn errors

/// Executes a shell command in a specified working directory.
///
/// IMPORTANT: This executes arbitrary commands. Callers MUST ensure commands are
/// appropriately sandboxed or confirmed by the user.
pub async fn execute_shell_command(
    command: &str,
    working_dir: &Path,
) -> Result<String> {
    debug!("Executing shell command: {} in {:?}", command, working_dir);

    // Use std::process::Command directly for shell execution
    let shell_executable = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let shell_arg = if cfg!(target_os = "windows") { "/C" } else { "-c" };

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
            // Return the error directly
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

    // Combine output for the final result string
    let result = format!(
        "Command executed: {} -c {}\nStatus: {}\nStdout:\n{}\nStderr:\n{}",
        shell_executable,
        command, // Show the command passed to the shell
        status,
        if stdout.is_empty() { "<no output>" } else { &stdout },
        if stderr.is_empty() { "<no output>" } else { &stderr }
    );

    Ok(result)
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
        println!("Output:\n{}", output);
        assert!(output.contains("Status: 0"));
        assert!(output.contains("\nStdout:\nHello Core Shell"));
        // Stderr check might vary slightly across OSes (e.g., empty vs <no output>)
        // assert!(output.contains("\nStderr:\n<no output>"));
    }

    #[tokio::test]
    async fn test_execute_shell_nonexistent_command() {
        let command = "this_command_does_not_exist_qwertyuiop";
        let working_dir = test_working_dir();
        let result = execute_shell_command(command, &working_dir).await;
        assert!(result.is_ok(), "Expected Ok result even for non-zero exit status");
        let output = result.unwrap();
        println!("Output:\n{}", output);
        assert!(!output.contains("Status: 0")); // Should not be status 0
        assert!(output.contains("command not found") || output.contains("is not recognized")); // Check for common error messages
    }
}
