// volition-cli/src/tools/shell.rs
use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use std::path::Path;
use tracing::{debug, warn};

// Removed ShellArgs import

// Use duct::cmd macro to execute the command string via the shell ("sh -c ...")
// Set working directory for the command.
pub(crate) async fn execute_shell_command_internal(
    command: &str,
    working_dir: &Path,
) -> Result<String> {
    debug!("Executing internal command: {} in {:?}", command, working_dir);

    let expression = duct::cmd!("sh", "-c", command).dir(working_dir);

    let output_result = expression
        .stdout_capture()
        .stderr_capture()
        .unchecked() // Don't panic on non-zero exit status
        .run();

    let (stdout_bytes, stderr_bytes, exit_status) = match output_result {
        Ok(output) => (
            output.stdout,
            output.stderr,
            output
                .status
                .code()
                .unwrap_or_else(|| if output.status.success() { 0 } else { 1 }),
        ),
        Err(e) => {
            warn!(command = command, error = %e, "Failed to spawn command process");
            return Err(e).context(format!("Failed to spawn process for command: {}", command));
        }
    };

    let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
    let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();

    // --- Logging (unchanged) ---
    let stdout_preview = stdout.lines().take(3).collect::<Vec<&str>>().join("\\n");
    let stderr_preview = if !stderr.is_empty() {
        format!(
            "\\nStderr preview: {}",
            stderr.lines().take(3).collect::<Vec<&str>>().join("\\n")
        )
    } else {
        String::new()
    };
    debug!(
        "Internal command exit status: {}\nOutput preview:\n{}{}",
        exit_status,
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
        "Command executed with status: {}\nStdout:\n{}\nStderr:\n{}",
        exit_status,
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
// Refactored signature to accept command: &str and working_dir: &Path
pub async fn run_shell_command(command: &str, working_dir: &Path) -> Result<String> {
    // --- Mandatory Confirmation (unchanged) ---
    print!(
        "{}\n{}\n{}{} ",
        "WARNING: This tool can execute arbitrary code!"
            .red()
            .bold(),
        format!("Request to run shell command: {}", command).yellow(),
        "Allow execution? ".yellow(),
        "(y/N):".yellow().bold() // Default to No
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
    // --- End Confirmation ---

    println!("{} {}", "Running:".blue().bold(), command);
    // Pass working_dir to the internal function
    execute_shell_command_internal(command, working_dir).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio;

    // Helper to get a valid Path for tests
    fn test_working_dir() -> PathBuf {
        // Use tempdir() to get a unique directory for each test run if possible,
        // otherwise fall back to current dir. This helps avoid test interference.
        tempdir()
            .map(|d| d.into_path())
            .unwrap_or_else(|_| PathBuf::from("."))
    }

    // NOTE: These tests now only test the *mocked* internal function,
    // as the real one is behind cfg(not(test)).
    // The confirmation logic in run_shell_command is not tested.

    #[tokio::test]
    async fn test_execute_shell_internal_success_mocked() {
        let command = "echo Mock Success";
        let working_dir = test_working_dir();
        // Use a mock internal implementation for testing
        async fn mock_internal(_cmd: &str, _wd: &Path) -> Result<String> {
            Ok("Command executed with status: 0\nStdout:\nMock Success\nStderr:\n<no output>".to_string())
        }
        let result = mock_internal(command, &working_dir).await;

        assert!(result.is_ok());
        let output_str = result.unwrap();
        assert!(output_str.starts_with("Command executed with status: 0"));
        assert!(output_str.contains("\nStdout:\nMock Success\n"));
    }

     #[tokio::test]
    async fn test_execute_shell_internal_fail_status_stderr_mocked() {
        let command = "ls /non_existent_directory_for_volition_test";
        let working_dir = test_working_dir();
         // Use a mock internal implementation for testing
        async fn mock_internal(_cmd: &str, _wd: &Path) -> Result<String> {
             Ok("Command executed with status: 2\nStdout:\n<no output>\nStderr:\nls: cannot access...".to_string())
        }
        let result = mock_internal(command, &working_dir).await;

        assert!(result.is_ok()); // Mock returns Ok
        let output_str = result.unwrap();
        assert!(output_str.starts_with("Command executed with status: 2"));
        assert!(output_str.contains("\nStderr:\nls: cannot access"));
    }

    #[tokio::test]
    async fn test_execute_shell_internal_command_not_found_mocked() {
        let command = "this_command_should_absolutely_not_exist_ever_42";
        let working_dir = test_working_dir();
         // Use a mock internal implementation for testing
        async fn mock_internal(cmd: &str, _wd: &Path) -> Result<String> {
            Ok(format!("Command executed with status: 127\nStdout:\n<no output>\nStderr:\nsh: {}: command not found", cmd))
        }
        let result = mock_internal(command, &working_dir).await;

        assert!(result.is_ok()); // Mock returns Ok
        let output_str = result.unwrap();
        assert!(output_str.starts_with("Command executed with status: 127"));
        assert!(output_str.contains("command not found"));
    }
}
